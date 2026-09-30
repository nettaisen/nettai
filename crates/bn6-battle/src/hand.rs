//! A player's chip hand for the current turn (the game's per-player "chip
//! block"). See docs/engine/chips.md §2.

use crate::battle::Battle;
use crate::content::{ChipFlags, ChipId};

/// Up to five chips, in use order, with their build-time damage and bonuses.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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

    /// The next chip, if any.
    pub fn next_chip(&self) -> Option<ChipId> {
        let id = *self.ids.get(self.cursor as usize)?;
        (id != NO_CHIP).then_some(id)
    }

    /// `sub_800FC7C`: move on to the next chip (not past the last).
    pub fn advance(&mut self) {
        if self.cursor < 5 && self.ids[self.cursor as usize] != NO_CHIP {
            self.cursor += 1;
        }
    }

    /// `sub_80108FC`: from the cursor on, the link navis' own chips
    /// (0x190..=0x19A) leave the hand; the entries after each move up one
    /// (`sub_801092C`), the last staying where it was.
    pub fn drop_link_navi_chips(&mut self) {
        let mut i = self.cursor as usize;
        let mut removed = 0;
        while let Some(&id) = self.ids.get(i) {
            if id == NO_CHIP {
                return;
            }
            if !(0x190..=0x19A).contains(&id) {
                i += 1;
                continue;
            }
            removed += 1;
            if removed > self.ids.len() {
                panic!("dropping the link navis' chips from the hand loops forever (sub_80108FC)");
            }
            for j in i..self.ids.len() - 1 {
                self.ids[j] = self.ids[j + 1];
                self.damage[j] = self.damage[j + 1];
                self.attack_bonus[j] = self.attack_bonus[j + 1];
                self.charge_bonus[j] = self.charge_bonus[j + 1];
                self.selection[j] = self.selection[j + 1];
                self.turn[j] = self.turn[j + 1];
                self.modifiers[j] = self.modifiers[j + 1];
            }
        }
        panic!("dropping the link navis' chips reads past the hand (sub_80108FC)");
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
    let d = b.content.chip(id).damage;
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
    if id == NO_CHIP || !b.content.chip(id).flags.has(ChipFlags::VARIABLE_DAMAGE) {
        return;
    }
    let d = chip_damage(b, id, side);
    b.hands[side as usize].damage[i] = d;
}
