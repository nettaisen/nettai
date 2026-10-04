//! Byte codecs for the engine's types that travel between peers: a recorded
//! custom-screen result in a tick's payload (`battle::PlayerInput`), and what
//! a player brings to a match in the handshake (a folder, the game, the SP
//! deletion times).
//!
//! Integers above a byte are LEB128 (rennet's varints), handles by their
//! index in the content: the handshake has checked that both peers run the
//! same content, so a handle names the same thing on both. Structs are
//! written field by field, destructured without `..`, so a new field doesn't
//! compile until it is written here too.

use std::io;

use nettai_battle::custom::{FolderChip, GameVersion, Recorded, SavedFolder};
use nettai_battle::setup::SpTimes;
use nettai_battle::hand::ChipHand;
use nettai_battle::setup::{GaugeSpeed, NaviCustBugs, NaviWeapons, Supports};
use nettai_battle::tactics::{MAX_ENTRIES, MAX_PATTERN_CHIPS, MAX_PATTERNS, Tactic, TacticPattern, Tactics};
use nettai_battle::transform::TransformRequest;
use nettai_battle::{ContentHash, CustomResult, NaviStats};
use nettai_battle::content::ChipCode;
use nettai_battle::patch_cards::InstalledCard;
use nettai_content_api::{ChipHandle, FormHandle, NaviHandle, PatchCardHandle, RecordHandle, StageHandle, WeaponHandle};

use crate::protocol::invalid;

/// Writes values to a byte buffer.
pub struct Writer<'a>(pub &'a mut Vec<u8>);

impl Writer<'_> {
    pub fn byte(&mut self, b: u8) {
        self.0.push(b);
    }

    pub fn uvarint(&mut self, v: u64) {
        rennet::write_uvarint(self.0, v).expect("writing to a Vec");
    }

    pub fn bytes(&mut self, b: &[u8]) {
        self.uvarint(b.len() as u64);
        self.0.extend_from_slice(b);
    }

    pub fn put<T: Wire>(&mut self, v: &T) {
        v.write(self);
    }
}

/// Reads values back from bytes.
pub struct Reader<'a> {
    rest: &'a [u8],
}

impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Reader<'a> {
        Reader { rest: bytes }
    }

    pub fn byte(&mut self) -> io::Result<u8> {
        let (&b, rest) = self.rest.split_first().ok_or_else(|| io::Error::from(io::ErrorKind::UnexpectedEof))?;
        self.rest = rest;
        Ok(b)
    }

    pub fn uvarint(&mut self) -> io::Result<u64> {
        rennet::read_uvarint(&mut self.rest)
    }

    pub fn bytes(&mut self) -> io::Result<&'a [u8]> {
        let n = self.uvarint()?;
        if n > self.rest.len() as u64 {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
        }
        let (b, rest) = self.rest.split_at(n as usize);
        self.rest = rest;
        Ok(b)
    }

    pub fn get<T: Wire>(&mut self) -> io::Result<T> {
        T::read(self)
    }

    /// The end: every byte was read.
    pub fn finish(self) -> io::Result<()> {
        if self.rest.is_empty() { Ok(()) } else { Err(invalid("bytes left over")) }
    }

    pub fn is_empty(&self) -> bool {
        self.rest.is_empty()
    }
}

/// A value with a byte form.
pub trait Wire: Sized {
    fn write(&self, w: &mut Writer);
    fn read(r: &mut Reader) -> io::Result<Self>;
}

/// The value encoded alone.
pub fn to_bytes<T: Wire>(v: &T) -> Vec<u8> {
    let mut out = Vec::new();
    v.write(&mut Writer(&mut out));
    out
}

/// The value decoded from all of `bytes`.
pub fn from_bytes<T: Wire>(bytes: &[u8]) -> io::Result<T> {
    let mut r = Reader::new(bytes);
    let v = T::read(&mut r)?;
    r.finish()?;
    Ok(v)
}

impl Wire for u8 {
    fn write(&self, w: &mut Writer) {
        w.byte(*self);
    }
    fn read(r: &mut Reader) -> io::Result<u8> {
        r.byte()
    }
}

/// A signed byte, as its byte (BN5's soul turns' bonus).
impl Wire for i8 {
    fn write(&self, w: &mut Writer) {
        w.byte(*self as u8);
    }
    fn read(r: &mut Reader) -> io::Result<i8> {
        Ok(r.byte()? as i8)
    }
}

impl Wire for bool {
    fn write(&self, w: &mut Writer) {
        w.byte(*self as u8);
    }
    fn read(r: &mut Reader) -> io::Result<bool> {
        match r.byte()? {
            0 => Ok(false),
            1 => Ok(true),
            b => Err(invalid(&format!("bad bool {b}"))),
        }
    }
}

macro_rules! wire_uint {
    ($($t:ty),*) => {$(
        impl Wire for $t {
            fn write(&self, w: &mut Writer) {
                w.uvarint(*self as u64);
            }
            fn read(r: &mut Reader) -> io::Result<$t> {
                <$t>::try_from(r.uvarint()?).map_err(|_| invalid(concat!(stringify!($t), " out of range")))
            }
        }
    )*};
}

wire_uint!(u16, u32, u64);

impl<T: Wire> Wire for Option<T> {
    fn write(&self, w: &mut Writer) {
        match self {
            None => w.byte(0),
            Some(v) => {
                w.byte(1);
                v.write(w);
            }
        }
    }
    fn read(r: &mut Reader) -> io::Result<Option<T>> {
        Ok(if r.get::<bool>()? { Some(r.get()?) } else { None })
    }
}

impl<T: Wire, const N: usize> Wire for [T; N] {
    fn write(&self, w: &mut Writer) {
        for v in self {
            v.write(w);
        }
    }
    fn read(r: &mut Reader) -> io::Result<[T; N]> {
        let v: Vec<T> = (0..N).map(|_| r.get()).collect::<io::Result<_>>()?;
        v.try_into().map_err(|_| unreachable!("N values were read"))
    }
}

impl<A: Wire, B: Wire> Wire for (A, B) {
    fn write(&self, w: &mut Writer) {
        self.0.write(w);
        self.1.write(w);
    }
    fn read(r: &mut Reader) -> io::Result<(A, B)> {
        Ok((r.get()?, r.get()?))
    }
}

impl<T: Wire> Wire for Vec<T> {
    fn write(&self, w: &mut Writer) {
        w.uvarint(self.len() as u64);
        for v in self {
            v.write(w);
        }
    }
    fn read(r: &mut Reader) -> io::Result<Vec<T>> {
        let n = r.uvarint()?;
        // Each value takes a byte at least: a longer count is malformed.
        if n > r.rest.len() as u64 {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
        }
        (0..n).map(|_| r.get()).collect()
    }
}

impl Wire for String {
    fn write(&self, w: &mut Writer) {
        w.bytes(self.as_bytes());
    }
    fn read(r: &mut Reader) -> io::Result<String> {
        String::from_utf8(r.bytes()?.to_vec()).map_err(|_| invalid("a string that isn't UTF-8"))
    }
}

macro_rules! wire_newtype {
    ($($t:ident($inner:ty)),* $(,)?) => {$(
        impl Wire for $t {
            fn write(&self, w: &mut Writer) {
                self.0.write(w);
            }
            fn read(r: &mut Reader) -> io::Result<$t> {
                Ok($t(r.get::<$inner>()?))
            }
        }
    )*};
}

wire_newtype!(
    ChipHandle(u16),
    FormHandle(u16),
    NaviHandle(u16),
    WeaponHandle(u16),
    RecordHandle(u16),
    PatchCardHandle(u16),
    StageHandle(u16),
    ChipCode(u8),
);

impl Wire for ContentHash {
    fn write(&self, w: &mut Writer) {
        w.0.extend_from_slice(&self.0.to_le_bytes());
    }
    fn read(r: &mut Reader) -> io::Result<ContentHash> {
        let b: [u8; 8] = r.get()?;
        Ok(ContentHash(u64::from_le_bytes(b)))
    }
}

/// A struct, field by field (all of them: no `..`).
macro_rules! wire_struct {
    ($($t:ty { $($f:ident),* $(,)? })*) => {$(
        impl Wire for $t {
            fn write(&self, w: &mut Writer) {
                let Self { $($f),* } = self;
                $( $f.write(w); )*
            }
            fn read(r: &mut Reader) -> io::Result<Self> {
                Ok(Self { $($f: r.get()?),* })
            }
        }
    )*};
}

wire_struct! {
    FolderChip { id, code }
    SavedFolder { chips, regular, tags }
    TransformRequest { form, navi_switch, turns, chaos }
    Supports { rush, beat, tango }
    NaviWeapons { buster, charge_shot, back_special, a_charge, mode9_a, buster_shot, charge_shot_kind, back_special_damage }
    NaviCustBugs {
        auto_step, panel_trail_kind, panel_trail_level, buster_blanks, buster_charged, hit_status, hp_drain,
        custom_drain, battle_start, emotion, processing, starting_damage, status_immunity, custom_damage,
        hand_shrink_turn,
    }
    NaviStats {
        attack, rapid, charge, first_barrier, gauge_speed, reg_up, custom_level, mega_level, giga_level, support,
        mood, element, starting_form, float_shoes, air_shoes, undershirt, super_armor, version, beast_out_counter,
        sun, chip_drops, encounters, navi, navi_variant, form, folder, folder_reg, max_base_hp, hp, max_hp,
        chip_recovery, folder_tags, chip_shuffle, number_open, hub_style, soul_turn_bonus, weapons, bugs,
    }
    ChipHand { cursor, ids, damage, attack_bonus, charge_bonus, selection, turn, modifiers }
    CustomResult { hand, navi_stats, transform }
}

impl Wire for GaugeSpeed {
    fn write(&self, w: &mut Writer) {
        w.byte(*self as u8);
    }
    fn read(r: &mut Reader) -> io::Result<GaugeSpeed> {
        Ok(match r.byte()? {
            0 => GaugeSpeed::Normal,
            1 => GaugeSpeed::Fast,
            2 => GaugeSpeed::Slow,
            b => return Err(invalid(&format!("bad gauge speed {b}"))),
        })
    }
}

impl Wire for GameVersion {
    fn write(&self, w: &mut Writer) {
        w.byte(match self {
            GameVersion::Gregar => 0,
            GameVersion::Falzar => 1,
        });
    }
    fn read(r: &mut Reader) -> io::Result<GameVersion> {
        Ok(match r.byte()? {
            0 => GameVersion::Gregar,
            1 => GameVersion::Falzar,
            b => return Err(invalid(&format!("bad game {b}"))),
        })
    }
}

impl Wire for InstalledCard {
    fn write(&self, w: &mut Writer) {
        self.card.write(w);
        self.enabled.write(w);
    }
    fn read(r: &mut Reader) -> io::Result<InstalledCard> {
        Ok(InstalledCard { card: r.get()?, enabled: r.get()? })
    }
}

/// A tactics entry: its kind's byte (0 a chip, 1 a pattern, 2 nothing, 3 an
/// empty place), then a chip's handle or a pattern's index.
impl Wire for Tactic {
    fn write(&self, w: &mut Writer) {
        match *self {
            Tactic::Chip(c) => {
                0u8.write(w);
                c.write(w);
            }
            Tactic::Pattern(i) => {
                1u8.write(w);
                i.write(w);
            }
            Tactic::Nothing => 2u8.write(w),
            Tactic::Empty => 3u8.write(w),
        }
    }
    fn read(r: &mut Reader) -> io::Result<Tactic> {
        Ok(match r.get::<u8>()? {
            0 => Tactic::Chip(r.get()?),
            1 => Tactic::Pattern(r.get()?),
            2 => Tactic::Nothing,
            3 => Tactic::Empty,
            _ => return Err(invalid("a tactics entry of no kind")),
        })
    }
}

/// A pattern: its place (each a signed byte) and its chips.
impl Wire for TacticPattern {
    fn write(&self, w: &mut Writer) {
        let TacticPattern { dx, dy, chips } = self;
        (*dx as u8).write(w);
        (*dy as u8).write(w);
        chips.write(w);
    }
    fn read(r: &mut Reader) -> io::Result<TacticPattern> {
        let dx = r.get::<u8>()? as i8;
        let dy = r.get::<u8>()? as i8;
        let chips: Vec<ChipHandle> = r.get()?;
        if chips.len() > MAX_PATTERN_CHIPS {
            return Err(invalid("a tactics pattern of too many chips"));
        }
        Ok(TacticPattern { dx, dy, chips })
    }
}

impl Wire for Tactics {
    fn write(&self, w: &mut Writer) {
        let Tactics { entries, patterns } = self;
        entries.write(w);
        patterns.write(w);
    }
    fn read(r: &mut Reader) -> io::Result<Tactics> {
        let entries: Vec<Tactic> = r.get()?;
        let patterns: Vec<TacticPattern> = r.get()?;
        if entries.len() > MAX_ENTRIES || patterns.len() > MAX_PATTERNS {
            return Err(invalid("tactics past the block"));
        }
        Ok(Tactics { entries, patterns })
    }
}

/// The SP deletion times, a halfword each.
impl Wire for SpTimes {
    fn write(&self, w: &mut Writer) {
        let SpTimes(frames) = self;
        for f in frames {
            f.write(w);
        }
    }
    fn read(r: &mut Reader) -> io::Result<SpTimes> {
        let mut frames = [0u16; 20];
        for f in &mut frames {
            *f = r.get()?;
        }
        Ok(SpTimes(frames))
    }
}

/// A result's box is unboxed on the wire.
impl Wire for Recorded {
    fn write(&self, w: &mut Writer) {
        let Recorded { in_custom, result } = self;
        in_custom.write(w);
        result.as_deref().cloned().write(w);
    }
    fn read(r: &mut Reader) -> io::Result<Recorded> {
        Ok(Recorded { in_custom: r.get()?, result: r.get::<Option<CustomResult>>()?.map(Box::new) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_battle::content::testing;

    fn roundtrip<T: Wire + PartialEq + std::fmt::Debug>(v: &T) -> usize {
        let b = to_bytes(v);
        assert_eq!(&from_bytes::<T>(&b).unwrap(), v);
        // Cut short, it is an error.
        if !b.is_empty() {
            assert!(from_bytes::<T>(&b[..b.len() - 1]).is_err());
        }
        b.len()
    }

    /// A built hand and the stats of a navi with every field set round
    /// trip, as a recorded result.
    #[test]
    fn a_custom_result_roundtrips() {
        let content = testing::content();
        let mut stats = testing::stats(321);
        stats.first_barrier = Some(RecordHandle(3));
        stats.support = Some(Supports { rush: true, beat: false, tango: true });
        stats.weapons.buster_shot = Some(RecordHandle(9));
        stats.bugs.custom_damage = 0x1234;
        stats.folder_tags = [[1, 2], [3, 0xFF]];
        let mut hand = ChipHand::empty(&content);
        hand.ids[0] = Some(ChipHandle(17));
        hand.damage[0] = 300;
        hand.selection[1] = Some(FolderChip::new(ChipHandle(400), ChipCode::ASTERISK));
        hand.turn = [0, 1, 2, 3, 4, 5];
        let result = CustomResult { hand: Some(hand), navi_stats: stats, transform: TransformRequest { form: Some(FormHandle(4)), ..TransformRequest::NONE } };
        let n = roundtrip(&Recorded { in_custom: true, result: Some(Box::new(result)) });
        assert!(n < 200, "{n} bytes");
        assert_eq!(roundtrip(&Recorded { in_custom: false, result: None }), 2);
    }

    #[test]
    fn setup_parts_roundtrip() {
        let chips = std::array::from_fn(|i| FolderChip::new(ChipHandle(i as u16 * 7), ChipCode(i as u8 % 27)));
        roundtrip(&SavedFolder { chips, regular: Some(4), tags: Some((1, 2)) });
        roundtrip(&vec![FormHandle(1), FormHandle(7)]);
        roundtrip(&SpTimes(std::array::from_fn(|i| i as u16 * 600)));
        roundtrip(&GameVersion::Gregar);
        roundtrip(&ContentHash(0x0123_4567_89AB_CDEF));
        roundtrip(&vec![(RecordHandle(3), true), (RecordHandle(300), false)]);
        roundtrip(&vec![
            InstalledCard { card: PatchCardHandle(3), enabled: true },
            InstalledCard { card: PatchCardHandle(116), enabled: false },
        ]);
        roundtrip(&"nettai".to_string());
    }
}
