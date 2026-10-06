//! A match in binary: what a replay keeps of the match it plays, and what
//! a netplay offer carries. It is read against the content it was written
//! on, which whoever carries the bytes names beside them (a replay's head,
//! the netplay handshake: the game and the content's hash), and nothing in
//! it is a name: a definition is its handle in the content (the pack's
//! index, the same on every load of the content the hash names), a
//! background its number in the pack.
//!
//! ```text
//! place := stage (u16: its handle), background (u16: its number plus one; 0 the stage's own)
//! arena := the first round's place, the later rounds' two places
//! side  := the side's setup of the game's rules in its compact form (`Block::write_compact`:
//!          each fact in the setup's order, a list as its count and its entries)
//! match := seed (u32), arena, side 0, side 1
//! offer := what the host brings (a byte: 0 nothing, 1 a stage, 2 an arena), then the stage's
//!          handle (u16) or the arena, then the side
//! ```
//!
//! Numbers are little-endian. The arena's layout is the engine's (a game's
//! arena is a stage and a background a round, whatever the game); a side's
//! is its game's rules' setup, so a game's facts change no other game's
//! bytes. A reader refuses bytes that end early or run on, a handle past
//! the content's definitions, a background the pack hasn't, and a value no
//! fact may hold, each with where; whether what it read is a match the
//! game's rules allow is the checks' (`check`), as for a match file.

use crate::facts::Facts;
use crate::{Arena, Match, Place, Side, ids};
use nettai_battle::content::{BackgroundId, Content};
use nettai_content_api::{Block, FieldType, FieldValue, StageHandle};

/// The bytes of a stage and its background.
const PLACE: usize = 4;

/// A side, appended to `out`: its facts' block, compact.
pub fn put_side(content: &Content, side: &Side, out: &mut Vec<u8>) {
    let rules = content.defs.rules().expect("a match's content has rules");
    let block = side.facts.block().expect("a side of a game with rules has its facts");
    block.write_compact(content.defs.schema(rules.setup), out);
}

/// A side from the start of `bytes`, and how many bytes it took.
pub fn take_side(content: &Content, bytes: &[u8]) -> Result<(Side, usize), String> {
    let rules = content.defs.rules().ok_or_else(|| format!("{} has no rules", content.game()))?;
    let schema = content.defs.schema(rules.setup);
    let (block, used) = Block::read_compact(rules.setup, schema, bytes)?;
    for i in 0..schema.fields().len() {
        handles_held(content, &block, schema.place(i), &schema.field(i).name)?;
    }
    Ok((Side { facts: Facts::of_block(block) }, used))
}

/// That every definition the block holds at `place` (at any depth) is one
/// of the content's.
fn handles_held(content: &Content, block: &Block, place: nettai_content_api::Place, path: &str) -> Result<(), String> {
    match place.ty() {
        FieldType::Record(fields) => {
            for i in 0..fields.fields().len() {
                handles_held(content, block, place.field_at(i), &format!("{path}.{}", fields.field(i).name))?;
            }
        }
        FieldType::List(..) | FieldType::Array(..) => {
            for k in 0..block.len_at(place).expect("a list or an array") {
                handles_held(content, block, place.elem(k).expect("an element it holds"), &format!("{path}[{}]", k + 1))?;
            }
        }
        FieldType::Ref(registry, _) => {
            if let FieldValue::Ref(Some((r, h))) = block.get_at(place)
                && ids::key_of(content, r, h).is_none()
            {
                return Err(format!("{path}: no {registry} {h} in the content"));
            }
        }
        FieldType::Asset(kind) => {
            if let FieldValue::Asset(_, Some(h)) = block.get_at(place)
                && h as usize >= content.assets.names(*kind).len()
            {
                return Err(format!("{path}: no {kind} {h} in the pack"));
            }
        }
        _ => {}
    }
    Ok(())
}

/// An arena, appended to `out`; refused if a place's background is none
/// its game's pack has.
pub fn put_arena(content: &Content, arena: &Arena, out: &mut Vec<u8>) -> Result<(), String> {
    for (i, p) in std::iter::once(&arena.first).chain(&arena.later).enumerate() {
        out.extend_from_slice(&p.stage.0.to_le_bytes());
        let background = match &p.background {
            None => 0,
            Some(name) => {
                let id = crate::background(content, &arena.game, name).ok_or_else(|| crate::no_background(&round(i), &arena.game, name))?;
                id.0 + 1
            }
        };
        out.extend_from_slice(&background.to_le_bytes());
    }
    Ok(())
}

/// The place `i` of an arena's three, for a message.
fn round(i: usize) -> String {
    match i {
        0 => "arena".to_string(),
        _ => format!("arena: later round {}", i + 1),
    }
}

/// An arena of the content's game from the start of `bytes`, and how many
/// bytes it took.
pub fn take_arena(content: &Content, bytes: &[u8]) -> Result<(Arena, usize), String> {
    if bytes.len() < 3 * PLACE {
        return Err("arena: the bytes end inside it".into());
    }
    let game = content.game();
    let mut places = (0..3).map(|i| {
        let at = &bytes[i * PLACE..];
        let stage = u16::from_le_bytes([at[0], at[1]]);
        if stage as usize >= content.defs.stages.len() {
            return Err(format!("{}: no stage {stage} in the content", round(i)));
        }
        let background = match u16::from_le_bytes([at[2], at[3]]) {
            0 => None,
            n => Some(
                ids::background_name(content, BackgroundId(n - 1))
                    .ok_or_else(|| format!("{}: no background {} in {game}'s pack", round(i), n - 1))?
                    .to_string(),
            ),
        };
        Ok(Place { stage: StageHandle(stage), background })
    });
    let first = places.next().expect("three places")?;
    let later = [places.next().expect("three places")?, places.next().expect("three places")?];
    Ok((Arena { game: game.to_string(), first, later }, 3 * PLACE))
}

/// A match in binary: its seed (refused: a match that states none), its
/// arena, its sides.
pub fn match_bytes(content: &Content, m: &Match) -> Result<Vec<u8>, String> {
    let seed = m.seed.ok_or("a match kept in binary states its seed")?;
    let mut out = seed.to_le_bytes().to_vec();
    put_arena(content, &m.arena, &mut out)?;
    for side in &m.sides {
        put_side(content, side, &mut out);
    }
    Ok(out)
}

/// A match from all of `bytes`.
pub fn read_match(content: &Content, bytes: &[u8]) -> Result<Match, String> {
    let seed = bytes.get(..4).ok_or("seed: the bytes end inside it")?;
    let seed = u32::from_le_bytes(seed.try_into().expect("four bytes"));
    let mut at = 4;
    let (arena, used) = take_arena(content, &bytes[at..])?;
    at += used;
    let mut side = |name: &str| -> Result<Side, String> {
        let (s, used) = take_side(content, &bytes[at..]).map_err(|e| format!("{name}: {e}"))?;
        at += used;
        Ok(s)
    };
    let sides = [side("left")?, side("right")?];
    finished(bytes, at)?;
    Ok(Match { seed: Some(seed), arena, sides })
}

/// That `at` is the end of `bytes`.
fn finished(bytes: &[u8], at: usize) -> Result<(), String> {
    match bytes.len() - at {
        0 => Ok(()),
        n => Err(format!("{n} bytes after its end")),
    }
}

/// What a netplay offer brings besides its side: from the host, the arena
/// or a stage a match must be fought on, if any.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Brings {
    Nothing,
    Stage(StageHandle),
    Arena(Arena),
}

/// A netplay offer in binary: what it brings, then its side.
pub fn offer_bytes(content: &Content, brings: &Brings, side: &Side) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    match brings {
        Brings::Nothing => out.push(0),
        Brings::Stage(s) => {
            out.push(1);
            out.extend_from_slice(&s.0.to_le_bytes());
        }
        Brings::Arena(a) => {
            out.push(2);
            put_arena(content, a, &mut out)?;
        }
    }
    put_side(content, side, &mut out);
    Ok(out)
}

/// A netplay offer from all of `bytes`.
pub fn read_offer(content: &Content, bytes: &[u8]) -> Result<(Brings, Side), String> {
    let (&kind, rest) = bytes.split_first().ok_or("the bytes are empty")?;
    let mut at = 1;
    let brings = match kind {
        0 => Brings::Nothing,
        1 => {
            let s = rest.get(..2).ok_or("stage: the bytes end inside it")?;
            let stage = u16::from_le_bytes([s[0], s[1]]);
            if stage as usize >= content.defs.stages.len() {
                return Err(format!("stage: no stage {stage} in the content"));
            }
            at += 2;
            Brings::Stage(StageHandle(stage))
        }
        2 => {
            let (arena, used) = take_arena(content, rest)?;
            at += used;
            Brings::Arena(arena)
        }
        n => return Err(format!("what the host brings is {n}: none of nothing (0), a stage (1), an arena (2)")),
    };
    let (side, used) = take_side(content, &bytes[at..]).map_err(|e| format!("side: {e}"))?;
    at += used;
    finished(bytes, at)?;
    Ok((brings, side))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{exe5_content, exe6_content};

    /// Live play's random matches of both games written and read back: the
    /// same match, the same bytes again, a match the checks pass; and on
    /// another load of the content, the same bytes (a handle is the pack's
    /// index, the same on every load of content of one hash).
    #[test]
    fn a_match_reads_back_from_its_bytes() {
        for (content, game) in [(exe6_content(), "exe6"), (exe5_content(), "exe5")] {
            for seed in [1, 77, 4242] {
                let m = crate::pick::live(&content, game, seed, None).unwrap();
                let bytes = match_bytes(&content, &m).unwrap();
                let back = read_match(&content, &bytes).unwrap_or_else(|e| panic!("{game} {seed}: {e}"));
                assert_eq!(back, m, "{game} {seed}");
                assert_eq!(match_bytes(&content, &back).unwrap(), bytes, "{game} {seed}");
                assert_eq!(crate::check_match(&content, &back), Vec::<String>::new(), "{game} {seed}");
            }
        }
        let again = std::sync::Arc::new(crate::testing::defined(&["exe6"]).unwrap());
        assert_eq!(again.hash(), exe6_content().hash());
        let of = |c: &std::sync::Arc<Content>| match_bytes(c, &crate::pick::live(c, "exe6", 77, None).unwrap()).unwrap();
        assert_eq!(of(&again), of(&exe6_content()));
    }

    /// An offer of a random side and its host's arena reads back, and fits
    /// one datagram with room to spare, EXE5's auto battle data and all.
    #[test]
    fn an_offer_fits_a_datagram() {
        for (content, game) in [(exe6_content(), "exe6"), (exe5_content(), "exe5")] {
            let m = crate::pick::live(&content, game, 9, None).unwrap();
            for brings in [Brings::Nothing, Brings::Stage(m.arena.first.stage), Brings::Arena(m.arena.clone())] {
                let bytes = offer_bytes(&content, &brings, &m.sides[0]).unwrap();
                assert!(bytes.len() < 1200, "{game}: {} bytes", bytes.len());
                assert_eq!(read_offer(&content, &bytes).unwrap(), (brings, m.sides[0].clone()), "{game}");
            }
        }
    }

    /// What isn't a match is refused, saying where.
    #[test]
    fn bytes_that_arent_a_match_are_refused() {
        let content = exe6_content();
        let m = crate::pick::live(&content, "exe6", 5, None).unwrap();
        let bytes = match_bytes(&content, &m).unwrap();
        let read = |b: &[u8]| read_match(&content, b).unwrap_err();
        assert_eq!(read(&bytes[..3]), "seed: the bytes end inside it");
        assert_eq!(read(&bytes[..10]), "arena: the bytes end inside it");
        let cut = read(&bytes[..bytes.len() - 1]);
        assert!(cut.starts_with("right: ") && cut.ends_with("the bytes end inside it"), "{cut}");
        let mut more = bytes.clone();
        more.push(0);
        assert_eq!(read(&more), "1 bytes after its end");
        let mut stage = bytes.clone();
        stage[4..6].copy_from_slice(&u16::MAX.to_le_bytes());
        assert_eq!(read(&stage), "arena: no stage 65535 in the content");
        let mut background = bytes.clone();
        background[14..16].copy_from_slice(&u16::MAX.to_le_bytes());
        assert_eq!(read(&background), "arena: later round 3: no background 65534 in exe6's pack");
        // The left side's navi, a handle past the content's navis.
        let at = 4 + 3 * PLACE + before(&content, &m.sides[0], "navi");
        let mut navi = bytes.clone();
        navi[at..at + 2].copy_from_slice(&0x7FFFu16.to_le_bytes());
        assert_eq!(read(&navi), "left: navi: no navi 32766 in the content");
        let mut offer = offer_bytes(&content, &Brings::Nothing, &m.sides[0]).unwrap();
        offer[0] = 3;
        assert_eq!(read_offer(&content, &offer).unwrap_err(), "what the host brings is 3: none of nothing (0), a stage (1), an arena (2)");
    }

    /// Where fact `name` of a side starts in its compact form: the bytes of
    /// the facts before it.
    fn before(content: &Content, side: &Side, name: &str) -> usize {
        let schema = content.defs.schema(content.defs.rules().unwrap().setup);
        let block = side.facts.block().unwrap();
        (0..schema.index_of(name).unwrap()).map(|i| compact_len(block, schema.place(i))).sum()
    }

    fn compact_len(block: &Block, place: nettai_content_api::Place) -> usize {
        match place.ty() {
            FieldType::Record(fields) => (0..fields.fields().len()).map(|i| compact_len(block, place.field_at(i))).sum(),
            FieldType::List(_, max) => {
                (if *max > 255 { 2 } else { 1 }) + (0..block.len_at(place).unwrap()).map(|k| compact_len(block, place.elem(k).unwrap())).sum::<usize>()
            }
            ty => ty.size(),
        }
    }
}
