//! A player's patch cards (BN4's, EXE5's and EXE6's Modification Cards,
//! 改造カード; docs/engine/patch-cards.md): the cards their save has
//! installed, in their list's order, each switched on or off.
//!
//! The cards are definitions (`define.patch_card`, `Content::patch_card`);
//! a player's installed cards are their setup's
//! ([`crate::custom::PlayerSetup::patch_cards`]), which the setup exchange
//! and the setup's hash cover as the rest of it. What the cards do is a
//! game's rules': EXE6's patch cards part reads a side's cards
//! (`battle.patch_cards(side)`) and applies them to its stats as the round
//! is set up (the `round_setup` hook).

use nettai_content_api::PatchCardHandle;

/// How many cards a player can have installed: EXE6's save list's room (its
/// 80 MB allow 16, as each card takes 5 or more).
pub const MAX_CARDS: usize = 32;

/// A card in a player's list: switched off, it stays installed but doesn't
/// apply (EXE6's menu's toggle, bit 7 of its byte in the save's list).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InstalledCard {
    pub card: PatchCardHandle,
    pub enabled: bool,
}

/// A player's installed cards, in their list's order (the order they
/// apply in).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PatchCards {
    cards: [Option<InstalledCard>; MAX_CARDS],
    len: u8,
}

impl Default for PatchCards {
    fn default() -> PatchCards {
        PatchCards { cards: [None; MAX_CARDS], len: 0 }
    }
}

impl PatchCards {
    /// The list `cards`, in order; an error past [`MAX_CARDS`].
    pub fn new(cards: &[InstalledCard]) -> Result<PatchCards, String> {
        if cards.len() > MAX_CARDS {
            return Err(format!("{} patch cards installed: a player's list holds {MAX_CARDS}", cards.len()));
        }
        let mut list = PatchCards::default();
        for (slot, &c) in list.cards.iter_mut().zip(cards) {
            *slot = Some(c);
        }
        list.len = cards.len() as u8;
        Ok(list)
    }

    /// No card is installed.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn len(&self) -> usize {
        self.len as usize
    }

    /// The installed cards in order, switched off or not.
    pub fn iter(&self) -> impl Iterator<Item = InstalledCard> + '_ {
        self.cards.iter().take(self.len as usize).map(|c| c.expect("a card in the list"))
    }
}
