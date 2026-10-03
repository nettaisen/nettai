//! A player's chip hand for the current turn (the game's per-player "chip
//! block"). See docs/engine/chips.md §2.

use crate::battle::Battle;
use crate::content::ChipFlags;
use crate::custom::FolderChip;
use nettai_content_api::ChipHandle;

/// Up to five chips, in use order, with their build-time damage and bonuses.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ChipHand {
    /// Index of the next chip to use (0..=5).
    pub cursor: u8,
    /// The chips (after Program Advance and modifier folding), ended by the
    /// first empty entry (what lies past it is left from before).
    pub ids: [Option<ChipHandle>; 6],
    /// Base damage per entry.
    pub damage: [u16; 6],
    /// Atk+ / Navi+ bonus folded into each entry.
    pub attack_bonus: [u16; 6],
    /// Bonus raised while charging (some forms).
    pub charge_bonus: [u16; 6],
    /// The chips picked, with their codes, before folding; none past the
    /// picks (the game's 0xFFFF). A hand never built holds the zeroed
    /// block: chip 0 in code A.
    pub selection: [Option<FolderChip>; 6],
    /// Which turn (custom screen, from 0) each chip was picked in.
    pub turn: [u8; 6],
    /// Modifier flags per entry (bit 1: WhiCapsl folded, bit 2: Uninstll).
    pub modifiers: [u8; 6],
}

impl ChipHand {
    /// The hand every battle starts with: no chips (the selection zeroed:
    /// game `game`'s chip 0 in code A, the arena's).
    pub fn empty(content: &crate::content::Content, game: crate::content::RootId) -> ChipHand {
        let zeroed = content.zeroed_chip(game).map(|id| FolderChip::new(id, crate::content::ChipCode(0)));
        ChipHand {
            cursor: 0,
            ids: [None; 6],
            damage: [0; 6],
            attack_bonus: [0; 6],
            charge_bonus: [0; 6],
            selection: [zeroed; 6],
            turn: [0; 6],
            modifiers: [0; 6],
        }
    }

    /// The next chip, if any.
    pub fn next_chip(&self) -> Option<ChipHandle> {
        *self.ids.get(self.cursor as usize)?
    }

    /// `sub_800FC7C`: move on to the next chip (not past the last).
    pub fn advance(&mut self) {
        if self.cursor < 5 && self.ids[self.cursor as usize].is_some() {
            self.cursor += 1;
        }
    }

    /// `sub_80108FC`: from the cursor on, the link navis' own chips (the
    /// original's last block of chips) leave the hand; the entries after each move up one
    /// (`sub_801092C`), the last staying where it was.
    pub fn drop_link_navi_chips(&mut self, content: &crate::content::Content) {
        let mut i = self.cursor as usize;
        let mut removed = 0;
        while let Some(&id) = self.ids.get(i) {
            let Some(id) = id else { return };
            if content.chip_links(id).own_chip_of.is_none() {
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
        self.ids.iter().skip(self.cursor as usize).take_while(|id| id.is_some()).count() as u8
    }
}

/// `sub_80109A4`: a chip's damage, evaluating its damage formula if it
/// has one (the original's damage values of 1000 and up).
pub fn chip_damage(b: &Battle, id: Option<ChipHandle>, side: u8) -> u16 {
    let Some(id) = id else { return 0 };
    let c = b.content.chip(id);
    match &c.formula {
        None => c.damage,
        Some(f) => crate::kinds::chip_damage_formula(b, id, side, f),
    }
}

/// `chip_800AEE8`: the next chip's damage is recomputed every tick when its
/// data asks for it.
pub fn refresh_variable_damage(b: &mut Battle, side: u8) {
    let hand = &b.hands[side as usize];
    let i = hand.cursor as usize;
    let Some(&Some(id)) = hand.ids.get(i) else { return };
    if !b.content.chip(id).flags.has(ChipFlags::VARIABLE_DAMAGE) {
        return;
    }
    let d = chip_damage(b, Some(id), side);
    b.hands[side as usize].damage[i] = d;
}
