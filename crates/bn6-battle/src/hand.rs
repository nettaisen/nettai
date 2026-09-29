//! A player's chip hand for the current turn (the game's per-player "chip
//! block"). See docs/engine/chips.md §2.

use crate::battle::Battle;
use crate::data::{self, ChipFlags, ChipId};

/// Up to five chips, in use order, with their build-time damage and bonuses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChipHand {
    /// Index of the next chip to use (0..=5).
    pub cursor: u8,
    /// Effective chip ids (after Program Advance and modifier folding),
    /// `NO_CHIP`-terminated.
    pub ids: [ChipId; 6],
    /// Base damage per entry.
    pub damage: [u16; 6],
    /// Atk+ / Navi+ bonus folded into each entry.
    pub attack_bonus: [u16; 6],
    /// Bonus raised while charging (some forms).
    pub charge_bonus: [u16; 6],
    /// The raw selection (`code << 9 | id`), before folding.
    pub selection: [u16; 6],
    /// Which turn (custom screen, from 0) each chip was picked in.
    pub turn: [u8; 6],
    /// Modifier flags per entry (bit 1: WhiCapsl folded, bit 2: Uninstll).
    pub modifiers: [u8; 6],
}

pub const NO_CHIP: ChipId = 0xFFFF;

impl ChipHand {
    /// The hand every battle starts with: no chips.
    pub fn empty() -> ChipHand {
        ChipHand {
            cursor: 0,
            ids: [NO_CHIP; 6],
            damage: [0; 6],
            attack_bonus: [0; 6],
            charge_bonus: [0; 6],
            selection: [0; 6],
            turn: [0; 6],
            modifiers: [0; 6],
        }
    }

    /// Parse the game's 0x50-byte chip block encoding. Byte 1 is always 0
    /// in battle (only the battle flag 0x40 mode's unreferenced routines
    /// use it) and is not kept.
    pub fn from_bytes(b: &[u8]) -> ChipHand {
        let u16s = |off: usize| -> [u16; 6] { std::array::from_fn(|i| u16::from_le_bytes([b[off + 2 * i], b[off + 2 * i + 1]])) };
        ChipHand {
            cursor: b[0],
            ids: u16s(0x02),
            damage: u16s(0x0E),
            attack_bonus: u16s(0x1A),
            charge_bonus: u16s(0x26),
            selection: u16s(0x32),
            turn: b[0x3E..0x44].try_into().unwrap(),
            modifiers: b[0x44..0x4A].try_into().unwrap(),
        }
    }

    /// The game's 0x50-byte encoding (for comparison with traces).
    pub fn to_bytes(&self) -> [u8; 0x50] {
        let mut b = [0u8; 0x50];
        b[0] = self.cursor;
        let mut put = |off: usize, v: &[u16; 6]| {
            for (i, x) in v.iter().enumerate() {
                b[off + 2 * i..off + 2 * i + 2].copy_from_slice(&x.to_le_bytes());
            }
        };
        put(0x02, &self.ids);
        put(0x0E, &self.damage);
        put(0x1A, &self.attack_bonus);
        put(0x26, &self.charge_bonus);
        put(0x32, &self.selection);
        b[0x3E..0x44].copy_from_slice(&self.turn);
        b[0x44..0x4A].copy_from_slice(&self.modifiers);
        b
    }

    /// The next chip, if any.
    pub fn next_chip(&self) -> Option<ChipId> {
        let id = *self.ids.get(self.cursor as usize)?;
        (id != NO_CHIP).then_some(id)
    }

    /// Chips left, counting from the cursor.
    pub fn remaining(&self) -> u8 {
        self.ids.iter().skip(self.cursor as usize).take_while(|&&id| id != NO_CHIP).count() as u8
    }
}

/// `sub_80109A4`: a chip's damage, evaluating damage formulas (values of
/// 1000 and up).
pub fn chip_damage(b: &Battle, id: ChipId, side: u8) -> u16 {
    if id == NO_CHIP {
        return 0;
    }
    let d = data::chip(id).damage;
    if d < 1000 {
        return d;
    }
    crate::kinds::chip_damage_formula(b, id, side, d - 1000)
}

/// `chip_800AEE8`: the next chip's damage is recomputed every tick when its
/// data asks for it.
pub fn refresh_variable_damage(b: &mut Battle, side: u8) {
    let hand = &b.hands[side as usize];
    let i = hand.cursor as usize;
    let Some(&id) = hand.ids.get(i) else { return };
    if id == NO_CHIP || !data::chip(id).flags.has(ChipFlags::VARIABLE_DAMAGE) {
        return;
    }
    let d = chip_damage(b, id, side);
    b.hands[side as usize].damage[i] = d;
}
