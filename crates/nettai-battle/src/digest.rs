//! A stable digest of the simulation state, for desync detection under
//! rollback netplay (see docs/design/rollback.md).
//!
//! Every type that holds simulation state implements [`Hash`]. The digest
//! feeds that state through [`StableHasher`], whose output depends only on
//! the values hashed: integers of every width are mixed as little-endian
//! 64-bit words, whatever the platform's word size. Two builds of the same
//! engine agree on the digest on any little-endian target (all the targets
//! a frontend runs on: x86-64, AArch64, wasm32). Peers must run the same
//! engine build: the digest covers the state's layout, which changes with
//! the engine.
//!
//! Left out, because they are presentation (the simulation never reads
//! them, and a frontend may present them per viewer):
//!
//! - the tick's sound cues (`Battle::sound_cues`) and the pause its
//!   players asked a host for (`Battle::pause_requests`);
//! - how sprites are drawn ([`Look`](crate::object::sprite::Look)), and how
//!   the other side's console turns an effect (`effect::Vars::other_flip`);
//! - which banner is showing ([`Banner::id`](crate::hud::Banner::id)) and
//!   what a telop says ([`Banner::telop`](crate::hud::Banner::telop), from
//!   the controller's `Object::telop_chip`: that a block shows a telop is
//!   hashed), and the chip a player just used, which the other player's
//!   console names (`Battle::used_chips`: how long the name still shows is
//!   hashed);
//! - what each console shows of its navi's chips (`Battle::chip_hud`), the
//!   HUD parts a chip hid (`Battle::hud_hidden`), and
//!   the HUD's message (`Battle::message`), warning markers
//!   (`Battle::warnings`) and HP numbers (`Battle::hp_numbers`);
//! - the objects' `VISIBLE` header flag and who else sees them
//!   (`Object::sight`);
//! - the looks a side's rules set (`Battle::looks`: EXE5's Hub Style's
//!   faces and enemy name);
//! - the message of an engine error that stopped the battle
//!   (`RoundEnd::Error`; that it stopped is hashed);
//! - how many times content read each side's keys while dimmed
//!   (`Battle::dimmed_key_reads`, for an AI: the simulation never reads
//!   it), and the panels each side's attacks were presented on during the
//!   tick (`Battle::attacked_panels`, for an AI likewise).
//!
//! Also left out: the content (`Battle::content`), which never changes and
//! whose hash the round's setup carries (`RoundSetup::content`). (The
//! content's runtime is no part of a battle: `behavior`.)
//!
//! The impls below destructure their structs without `..`, so adding a
//! field to one of them fails to compile until the field is either hashed
//! or explicitly left out here.

use crate::battle::Battle;
use crate::hud::Banner;
use crate::object::sprite::Sprite;
use crate::object::{Object, ObjectFlags};
use std::hash::{Hash, Hasher};

// Byte slices of integers (`[u16]`, `[u32]`...) reach the hasher as their
// in-memory bytes. On a big-endian target those would differ.
const _: () = assert!(cfg!(target_endian = "little"), "the state digest assumes a little-endian target");

/// A 64-bit hasher with platform-independent output: each value is mixed
/// in as a little-endian 64-bit word with a folded 128-bit multiply.
#[derive(Clone, Debug)]
pub struct StableHasher {
    state: u64,
    words: u64,
}

impl Default for StableHasher {
    fn default() -> StableHasher {
        StableHasher::new()
    }
}

const SEED: u64 = 0x243F_6A88_85A3_08D3;
const MUL: u64 = 0x9E37_79B9_7F4A_7C15;

fn fold(a: u64, b: u64) -> u64 {
    let m = (a as u128).wrapping_mul(b as u128);
    (m as u64) ^ ((m >> 64) as u64)
}

impl StableHasher {
    pub fn new() -> StableHasher {
        StableHasher { state: SEED, words: 0 }
    }

    fn word(&mut self, w: u64) {
        self.state = fold(self.state ^ w, MUL);
        self.words = self.words.wrapping_add(1);
    }
}

impl Hasher for StableHasher {
    fn write(&mut self, bytes: &[u8]) {
        let (words, rest) = bytes.as_chunks::<8>();
        for &w in words {
            self.word(u64::from_le_bytes(w));
        }
        if !rest.is_empty() {
            let mut last = [0u8; 8];
            last[..rest.len()].copy_from_slice(rest);
            // The tail's length goes in the top byte, so that "ab" and
            // "ab\0" differ.
            last[7] ^= rest.len() as u8;
            self.word(u64::from_le_bytes(last));
        }
    }
    fn write_u8(&mut self, i: u8) {
        self.word(i as u64);
    }
    fn write_u16(&mut self, i: u16) {
        self.word(i as u64);
    }
    fn write_u32(&mut self, i: u32) {
        self.word(i as u64);
    }
    fn write_u64(&mut self, i: u64) {
        self.word(i);
    }
    fn write_u128(&mut self, i: u128) {
        self.word(i as u64);
        self.word((i >> 64) as u64);
    }
    fn write_usize(&mut self, i: usize) {
        self.word(i as u64);
    }
    fn write_i8(&mut self, i: i8) {
        self.word(i as i64 as u64);
    }
    fn write_i16(&mut self, i: i16) {
        self.word(i as i64 as u64);
    }
    fn write_i32(&mut self, i: i32) {
        self.word(i as i64 as u64);
    }
    fn write_i64(&mut self, i: i64) {
        self.word(i as u64);
    }
    fn write_isize(&mut self, i: isize) {
        self.word(i as i64 as u64);
    }
    fn finish(&self) -> u64 {
        fold(self.state ^ self.words, MUL ^ SEED)
    }
}

/// Hashes a slice of flag words (a `bitflags` type's, as `bits().to_ne_bytes()`)
/// as `Hash::hash_slice` hashes a slice of their integers: their bytes in one
/// write. A flag type held in an array implements `hash_slice` with it, so
/// that the array hashes as the array of integers it replaced (a derived
/// `Hash` would hash each word apart).
pub fn hash_bits_slice<H: Hasher, const N: usize>(words: impl ExactSizeIterator<Item = [u8; N]>, h: &mut H) {
    let mut stack = [0u8; 64];
    let mut heap = Vec::new();
    let len = words.len() * N;
    let bytes: &mut [u8] = if len <= stack.len() {
        &mut stack[..len]
    } else {
        heap.resize(len, 0);
        &mut heap
    };
    for (to, word) in bytes.chunks_exact_mut(N).zip(words) {
        to.copy_from_slice(&word);
    }
    h.write(bytes);
}

/// The digest of anything hashable, with [`StableHasher`].
pub fn stable_hash<T: Hash + ?Sized>(value: &T) -> u64 {
    let mut h = StableHasher::new();
    value.hash(&mut h);
    h.finish()
}

impl Battle {
    /// A digest of the simulation state (everything a tick reads, not the
    /// presentation-only parts listed in the module docs). Two peers that
    /// simulated the same frames from the same inputs have equal digests;
    /// comparing them each confirmed frame detects a desync.
    pub fn digest(&self) -> u64 {
        stable_hash(self)
    }
}

/// The simulation state of a battle (see the module docs for what is left
/// out).
impl Hash for Battle {
    fn hash<H: Hasher>(&self, h: &mut H) {
        let Battle {
            // Read-only data shared by snapshots: its identity is in the
            // setup (`RoundSetup::content`).
            content: _,
            setup,
            stats,
            party,
            rng,
            consoles,
            round,
            fight,
            gauge,
            banner,
            remote_telop,
            used_chips,
            chip_hud: _,
            hud_hidden: _,
            message: _,
            warnings: _,
            hp_numbers: _,
            window_navis: _,
            paused,
            inputs,
            switch_inputs,
            mic_levels,
            screen_choices,
            screen_cues_at: _,
            hands,
            transform_requests,
            turn_transforms,
            transform_seq,
            custom_reversion,
            given,
            objects,
            actors,
            collision,
            field,
            fade,
            fadein_queue,
            damage_carry,
            custom,
            sides,
            looks: _,
            side_stats,
            navi_hit_counts,
            linked,
            dimming,
            rules,
            sound: _,
            pause_asked: _,
            outcome,
            folder_check: _,
            limits: _,
            tool_answer: _,
            dimmed_key_reads: _,
            attacked: _,
        } = self;
        setup.hash(h);
        stats.hash(h);
        party.hash(h);
        rng.hash(h);
        consoles.hash(h);
        round.hash(h);
        fight.hash(h);
        gauge.hash(h);
        banner.hash(h);
        remote_telop.hash(h);
        // (How long each name still shows, not what it says: where telops
        // run apart a controller waits on the other player's.)
        used_chips.each_ref().map(|u| u.map(|u| u.ticks)).hash(h);
        paused.hash(h);
        inputs.hash(h);
        switch_inputs.hash(h);
        mic_levels.hash(h);
        screen_choices.hash(h);
        hands.hash(h);
        transform_requests.hash(h);
        turn_transforms.hash(h);
        transform_seq.hash(h);
        custom_reversion.hash(h);
        given.hash(h);
        objects.hash(h);
        actors.hash(h);
        collision.hash(h);
        field.hash(h);
        fade.hash(h);
        fadein_queue.hash(h);
        damage_carry.hash(h);
        custom.hash(h);
        sides.hash(h);
        side_stats.hash(h);
        navi_hit_counts.hash(h);
        linked.hash(h);
        dimming.hash(h);
        rules.hash(h);
        // An engine error's message is left out: it is for people, and a
        // content runtime's can quote addresses.
        match outcome {
            Some(crate::battle::RoundEnd::Error(_)) => "stopped".hash(h),
            outcome => outcome.hash(h),
        }
    }
}

/// An object, without its `VISIBLE` flag (presentation).
impl Hash for Object {
    fn hash<H: Hasher>(&self, h: &mut H) {
        let Object {
            flags: header,
            kind,
            params,
            state,
            action,
            phase,
            phase_init,
            slide_bounds,
            drag_step,
            element,
            slide_type,
            anim,
            anim_loaded,
            panel,
            future_panel,
            alliance,
            flip,
            prevent_anim,
            shake_timer,
            chips_held,
            slide_tiles,
            slide_dx,
            slide_dy,
            slide_timer,
            slide_state,
            timer,
            timer2,
            hp,
            max_hp,
            identity,
            chip,
            damage,
            stamina,
            shake_origin_x,
            shake_origin_z,
            pos,
            vel,
            related,
            second_overlay,
            collision,
            actor,
            saved_state,
            vars,
            telop_chip: _,
            sight: _,
            drive,
        } = self;
        (*header - ObjectFlags::VISIBLE).hash(h);
        kind.hash(h);
        params.hash(h);
        (state, action, phase, phase_init).hash(h);
        (slide_bounds, drag_step).hash(h);
        (element, slide_type, anim, anim_loaded).hash(h);
        (panel, future_panel, alliance, flip).hash(h);
        (prevent_anim, shake_timer, chips_held).hash(h);
        (slide_tiles, slide_dx, slide_dy, slide_timer, slide_state).hash(h);
        (timer, timer2, hp, max_hp, identity, chip, damage, stamina).hash(h);
        (shake_origin_x, shake_origin_z).hash(h);
        (pos, vel).hash(h);
        (related, second_overlay, collision, actor, saved_state).hash(h);
        vars.hash(h);
        drive.hash(h);
    }
}

/// A sprite's animation state, without its [`Look`](crate::object::sprite::Look).
impl Hash for Sprite {
    fn hash<H: Hasher>(&self, h: &mut H) {
        let Sprite { id, anim, steps, started, look: _ } = self;
        (id, anim, steps, started).hash(h);
    }
}

/// A banner's lifetime (flow code waits on it) and whether it is a telop
/// (which the flow and the controllers tell apart, where telops run apart
/// from the banners), without which banner it is, what the telop says, or
/// its steps (its game's, the content's).
impl Hash for Banner {
    fn hash<H: Hasher>(&self, h: &mut H) {
        let Banner { active, step, timer, holds, steps: _, id: _, telop } = self;
        (active, step, timer, holds, telop.is_some()).hash(h);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integers_hash_by_value_whatever_their_width() {
        // A usize is mixed as a 64-bit word on every platform.
        assert_eq!(stable_hash(&7usize), stable_hash(&7u64));
        assert_ne!(stable_hash(&7u64), stable_hash(&8u64));
        assert_ne!(stable_hash(&[1u8, 2]), stable_hash(&[1u8, 2, 0]));
    }

    #[test]
    fn the_hasher_is_fixed() {
        // Pinned: a change here changes every digest (and breaks netplay
        // between builds).
        assert_eq!(stable_hash(&(1u8, 0x1234u16, -1i32, true, [5u16, 6])), 0xc5b0_6c2d_cdf9_1f35);
    }

    /// The state's flag words are their integers: the same size, and the
    /// same digest alone and in an array (and a short array or a long one,
    /// past `hash_bits_slice`'s stack buffer).
    #[test]
    fn flag_words_hash_as_their_integers() {
        use crate::actor::{ActorStatus, Requests};
        use crate::battle::{BattleFlags, RemoteStatus};
        use crate::collision::{CollisionTypes, HitModifiers, StatusFlags, StatusRequests};
        use crate::custom::builder::Marks;
        use crate::field::PanelFlags;
        use crate::input::Keys;
        use crate::object::ObjectFlags;
        use crate::setup::BattleEffects;
        use std::mem::size_of;
        assert_eq!(
            [size_of::<StatusFlags>(), size_of::<StatusRequests>(), size_of::<CollisionTypes>(), size_of::<PanelFlags>()],
            [4; 4]
        );
        assert_eq!([size_of::<Requests>(), size_of::<ActorStatus>(), size_of::<BattleEffects>()], [4; 3]);
        assert_eq!([size_of::<BattleFlags>(), size_of::<Keys>()], [2; 2]);
        assert_eq!([size_of::<ObjectFlags>(), size_of::<RemoteStatus>(), size_of::<HitModifiers>(), size_of::<Marks>()], [1; 4]);
        let words = [0x8000_0001u32, 0x20, 0x0400_0000];
        assert_eq!(stable_hash(&StatusFlags::from_bits_retain(words[0])), stable_hash(&words[0]));
        assert_eq!(stable_hash(&words.map(CollisionTypes::from_bits_retain)), stable_hash(&words));
        let long: Vec<u32> = (0..40).map(|i| 1 << (i % 32)).collect();
        let typed: Vec<CollisionTypes> = long.iter().map(|&w| CollisionTypes::from_bits_retain(w)).collect();
        assert_eq!(stable_hash(&typed), stable_hash(&long));
        let bytes = [0x05u8, 0x80, 0x41, 0, 0xFF, 0x10];
        assert_eq!(stable_hash(&bytes.map(Marks::from_bits_retain)), stable_hash(&bytes));
        assert_eq!(stable_hash(&bytes.map(HitModifiers::from_bits_retain)), stable_hash(&bytes));
        assert_eq!(stable_hash(&[bytes[0], bytes[1]].map(RemoteStatus::from_bits_retain)), stable_hash(&[bytes[0], bytes[1]]));
        assert_eq!(stable_hash(&Keys::from_bits_retain(0xFC03)), stable_hash(&0xFC03u16));
    }
}
