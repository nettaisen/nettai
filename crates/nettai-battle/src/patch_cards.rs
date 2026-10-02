//! Patch cards (the Japanese games' Modification Cards, 改造カード;
//! docs/design/patch-cards.md, docs/engine/patch-cards.md): what a player
//! has installed, and their application at the round's start.
//!
//! In the original the cards change the save's NaviStats whenever the
//! NaviCust's stats are recomputed (the Japanese `reloadCurNaviStatBoosts`,
//! its card routine), so the init exchange already carries their effects.
//! Here a player's cards are part of the shared setup
//! ([`crate::custom::PlayerSetup::patch_cards`]), the setup's stats are the
//! navi's before them, and [`apply`] runs the ruleset's application (the
//! role `hooks.patch_cards`, rules/patch-cards.luau) on each side's stats
//! once, before anything copies them.

use crate::battle::Battle;
use nettai_content_api::{HookCall, PatchCardHandle, Value};

/// How many cards a player can have installed: the original refuses a card
/// past 80 MB, and the cards take 5 MB or more (its list has room for 32).
pub const MAX_CARDS: usize = 16;

/// A card in a player's list: switched off, it stays installed but doesn't
/// apply (the menu's toggle, bit 7 of its byte).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InstalledCard {
    pub card: PatchCardHandle,
    pub enabled: bool,
}

/// A player's installed cards, in their list's order (the order they
/// apply in).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PatchCards {
    cards: [Option<InstalledCard>; MAX_CARDS],
    len: u8,
}

impl PatchCards {
    /// The list `cards`, in order. Panics past [`MAX_CARDS`].
    pub fn new(cards: &[InstalledCard]) -> PatchCards {
        assert!(cards.len() <= MAX_CARDS, "{} patch cards installed: the MB limit allows {MAX_CARDS}", cards.len());
        let mut list = PatchCards::default();
        for (slot, &c) in list.cards.iter_mut().zip(cards) {
            *slot = Some(c);
        }
        list.len = cards.len() as u8;
        list
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

/// The round's start: each side's cards change its stats (the ruleset's
/// `hooks.patch_cards`, which also skips a navi that isn't MegaMan). With
/// cards installed, switched off or not, the console's emotion window
/// glitches by what the hook returns (the stats after the cards have a
/// NaviCust bug: the Japanese `sub_813BF1C` reads event flag 0x1723 then,
/// not 0x1720). The battle-start copy of the stats (`cross_stats`) is of
/// the stats after the cards.
pub(crate) fn apply(b: &mut Battle) {
    for side in 0..2u8 {
        if b.setup.players[side as usize].patch_cards.is_empty() {
            continue;
        }
        let hook = b.content.defs.roles.hook(crate::content::HookRole::PatchCards);
        let glitch = match crate::behavior::call_hook(b, hook, HookCall::RolePatchCards { side }) {
            Value::Bool(g) => g,
            v => panic!("content error: hooks.patch_cards returns whether the stats have a bug (a boolean), not {v:?}"),
        };
        b.consoles[side as usize].emotion_window_glitch = glitch;
        b.cross_stats[side as usize] = b.stats[side as usize];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario;
    use crate::setup::{GaugeSpeed, Supports};

    fn card(content: &crate::content::Content, key: &str, enabled: bool) -> InstalledCard {
        InstalledCard { card: content.defs.patch_card_by_key(key).unwrap_or_else(|| panic!("no card {key:?}")), enabled }
    }

    fn with_cards(cards: &[(&str, bool)], tweak: impl FnOnce(&mut crate::setup::NaviStats)) -> Battle {
        let content = scenario::content();
        let mut s = scenario::setup();
        let list: Vec<InstalledCard> = cards.iter().map(|&(k, on)| card(&content, k, on)).collect();
        s.players[0].patch_cards = PatchCards::new(&list);
        tweak(&mut s.navi_stats[0]);
        Battle::new(s, content)
    }

    #[test]
    fn a_card_changes_the_stats_by_its_kinds_order() {
        let b = with_cards(&[("test-stats", true)], |_| {});
        let s = &b.stats[0];
        // HP 1000: +30 first, then +10% (the card lists them the other way).
        assert_eq!((s.max_hp, s.hp), (1133, 1133));
        assert_eq!((s.attack, s.element, s.bugs.hp_drain), (3, 2, 2));
        assert_eq!(b.cross_stats[0], b.stats[0], "the battle-start copy is of the stats after the cards");
        assert!(b.consoles[0].emotion_window_glitch, "the HP drain is a bug: flag 0x1723");
        assert_eq!(b.stats[1], scenario::setup().navi_stats[1], "the other side has none");
    }

    #[test]
    fn a_later_card_writes_over_an_earlier_one() {
        let b = with_cards(&[("test-stats", true), ("test-later", true)], |_| {});
        let s = &b.stats[0];
        assert_eq!(s.attack, 2, "Attack 0 + 3 - 1");
        assert_eq!(s.giga_level, 0xFF, "GigaFolder- doesn't clamp");
    }

    #[test]
    fn abilities_choices_and_chip_shuffle() {
        let b = with_cards(&[("test-abilities", true)], |s| {
            s.support = Some(Supports::default());
            s.float_shoes = true;
            s.number_open = true;
        });
        let s = &b.stats[0];
        let content = &b.content;
        assert!(s.super_armor && !s.float_shoes);
        assert_eq!(s.first_barrier, content.defs.record("barrier/200"));
        assert_eq!(s.weapons.charge_shot_kind, content.defs.record("shot/charged-confusing"));
        assert_eq!(s.support, Some(Supports { rush: true, ..Supports::default() }));
        assert_eq!(s.gauge_speed, GaugeSpeed::Fast);
        assert!(s.chip_shuffle && !s.number_open, "ChpShufl turns NumbrOpn off");
        assert!(!b.consoles[0].emotion_window_glitch, "no bug");
    }

    #[test]
    fn a_switched_off_card_does_nothing_but_the_glitch_follows_the_stats() {
        let b = with_cards(&[("test-stats", false)], |s| s.support = Some(Supports::default()));
        let mut want = scenario::setup().navi_stats[0];
        want.support = Some(Supports::default());
        // The HP is set to its maximum (the reload's, in the real world).
        want.hp = want.max_hp;
        assert_eq!(b.stats[0], want);
        assert!(!b.consoles[0].emotion_window_glitch);
        let bugged = with_cards(&[("test-stats", false)], |s| {
            s.support = Some(Supports::default());
            s.bugs.emotion = 1;
        });
        assert!(bugged.consoles[0].emotion_window_glitch, "a NaviCust bug counts with cards installed");
    }

    #[test]
    fn the_support_bug_keeps_supports_off() {
        let b = with_cards(&[("test-abilities", true)], |s| s.support = None);
        assert_eq!(b.stats[0].support, None, "the byte 0xFF stays 0xFF when a bit is set");
    }
}
