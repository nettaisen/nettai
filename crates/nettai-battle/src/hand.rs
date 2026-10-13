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
    /// Each entry's marks (the original's modifier flags): 0x01 the folder's
    /// Regular chip; the rest by game (the builder's folded modifier chips,
    /// a screen's marked picks, what the rules add), and what they do at
    /// the chip's use by the game's chip-use rules (`mark_damage`,
    /// `mark_heals`).
    pub modifiers: [u8; 6],
}

impl ChipHand {
    /// The hand every battle starts with: no chips (the selection zeroed:
    /// the game's chip 0 in code A).
    pub fn empty(content: &crate::content::Content) -> ChipHand {
        let zeroed = content.zeroed_chip().map(|id| FolderChip::new(id, crate::content::ChipCode(0)));
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

    /// EXE4's WoodSoul's folds: the chip after `i` folds into the chip at
    /// `i`, its damage and bonuses added (wrapping), and leaves the hand:
    /// the entries after it move up one, through the first empty one.
    ///
    /// `whole`: the status reset's (0x0800E294, on the hand: the damage,
    /// the Atk+ bonus and the charge bonus added; every per-slot field
    /// moves, 0x0800E2F0, the turn and mark bytes as `move_flag_bytes`
    /// says). Else the custom screen's as OK builds
    /// the hand (0x0801F242, on its list at 0x02033000: the damage and the
    /// Atk+ bonus added; the chips move up with their damage alone, it
    /// copies the list's +12, +36 and +48: each slot's Atk+ bonus and
    /// marks, +24 and +60, stay where they were, so a chip that moved up
    /// has the bonus of the one before it).
    pub fn fold_next(&mut self, i: usize, whole: bool) {
        let j = i + 1;
        assert!(self.ids.get(j).is_some_and(|c| c.is_some()), "folding into hand entry {i}: no chip after it (0x0800E294)");
        self.damage[i] = self.damage[i].wrapping_add(self.damage[j]);
        self.attack_bonus[i] = self.attack_bonus[i].wrapping_add(self.attack_bonus[j]);
        if whole {
            self.charge_bonus[i] = self.charge_bonus[i].wrapping_add(self.charge_bonus[j]);
        }
        let mut k = j;
        loop {
            let next = k + 1;
            if next >= self.ids.len() {
                panic!("folding the hand reads past it (0x0800E2F0, 0x0801F280)");
            }
            self.ids[k] = self.ids[next];
            self.damage[k] = self.damage[next];
            if whole {
                self.attack_bonus[k] = self.attack_bonus[next];
                self.charge_bonus[k] = self.charge_bonus[next];
                self.selection[k] = self.selection[next];
                self.move_flag_bytes(k);
            }
            if self.ids[k].is_none() {
                return;
            }
            k = next;
        }
    }

    /// What EXE4's fold on the hand does to the turn and mark bytes as
    /// entry `k` takes the next one's fields (0x0800E302, 0x0800E308). It
    /// reads and writes them through the entry's pointer, which steps two
    /// bytes an entry where these are a byte each: the byte at twice the
    /// entry's index takes the one two on, in the block's six turns
    /// (+0x3E) and again in its six marks (+0x44). The turns run on into
    /// the marks (an install's mark turns up in a turn byte), and the marks
    /// into the block's six unused bytes after them, which are zero.
    fn move_flag_bytes(&mut self, k: usize) {
        let mut bytes = [0u8; 18];
        bytes[..6].copy_from_slice(&self.turn);
        bytes[6..12].copy_from_slice(&self.modifiers);
        bytes[2 * k] = bytes[2 * k + 2];
        bytes[6 + 2 * k] = bytes[6 + 2 * k + 2];
        self.turn.copy_from_slice(&bytes[..6]);
        self.modifiers.copy_from_slice(&bytes[6..12]);
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
