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
//! - the tick's sound cues (`Battle::sound_cues`);
//! - how sprites are drawn ([`Look`](crate::object::sprite::Look));
//! - which banner is showing ([`Banner::id`](crate::hud::Banner::id)) and
//!   what a telop says ([`Banner::telop`](crate::hud::Banner::telop), from
//!   the controller's `Object::telop_chip`), and the chip a player just
//!   used, which the other player's console names (`Battle::used_chips`);
//! - what each console shows of its navi's chips (`Battle::chip_hud`), the
//!   HUD parts a chip hid (`Battle::hud_hidden`), and
//!   the HUD's message (`Battle::message`), warning markers
//!   (`Battle::warnings`) and HP numbers (`Battle::hp_numbers`);
//! - the objects' `VISIBLE` header flag and who else sees them
//!   (`Object::sight`);
//! - the message of an engine error that stopped the battle
//!   (`RoundEnd::Error`; that it stopped is hashed).
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
use crate::object::{Object, flags};
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
            // (Made from the setup and the content.)
            games: _,
            setup,
            stats,
            reserves,
            rng,
            consoles,
            round,
            fight,
            gauge,
            banner,
            used_chips: _,
            chip_hud: _,
            hud_hidden: _,
            message: _,
            warnings: _,
            hp_numbers: _,
            paused,
            inputs,
            hands,
            transform_requests,
            turn_transforms,
            transform_seq,
            custom_reversion,
            bug_frags,
            navi_levels,
            objects,
            actors,
            collision,
            field,
            fade,
            fadein_queue,
            damage_carry,
            custom,
            link,
            sides,
            side_stats,
            navi_hit_counts,
            tactics,
            linked,
            dimming,
            rules,
            sound: _,
            outcome,
            folder_check: _,
        } = self;
        setup.hash(h);
        stats.hash(h);
        reserves.hash(h);
        rng.hash(h);
        consoles.hash(h);
        round.hash(h);
        fight.hash(h);
        gauge.hash(h);
        banner.hash(h);
        paused.hash(h);
        inputs.hash(h);
        hands.hash(h);
        transform_requests.hash(h);
        turn_transforms.hash(h);
        transform_seq.hash(h);
        custom_reversion.hash(h);
        bug_frags.hash(h);
        navi_levels.hash(h);
        objects.hash(h);
        actors.hash(h);
        collision.hash(h);
        field.hash(h);
        fade.hash(h);
        fadein_queue.hash(h);
        damage_carry.hash(h);
        custom.hash(h);
        link.hash(h);
        sides.hash(h);
        side_stats.hash(h);
        navi_hit_counts.hash(h);
        tactics.hash(h);
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
        } = self;
        (header & !flags::VISIBLE).hash(h);
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
    }
}

/// A sprite's animation state, without its [`Look`](crate::object::sprite::Look).
impl Hash for Sprite {
    fn hash<H: Hasher>(&self, h: &mut H) {
        let Sprite { id, anim, frame, count, frame_flags, look: _ } = self;
        (id, anim, frame, count, frame_flags).hash(h);
    }
}

/// A banner's lifetime (flow code waits on it), without which banner it is.
impl Hash for Banner {
    fn hash<H: Hasher>(&self, h: &mut H) {
        let Banner { active, step, timer, holds, id: _, telop: _ } = self;
        (active, step, timer, holds).hash(h);
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
}
