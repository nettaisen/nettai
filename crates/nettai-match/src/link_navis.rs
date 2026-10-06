//! A link navi's stats at its level: EXE6's reload
//! (`reloadCurNaviBaseStats_8120df0`, with the HP as
//! `reloadCurNaviStatBoosts_813c3ac` leaves it), which the PET runs when a
//! navi code is received or the navi switched, before any battle. Its
//! tables are the navi definitions' `levels` (content/exe6/navis/*/init.luau);
//! docs/engine/link-navis.md has the routines and how the level is set.
//!
//! A side states its navi and its level, and no stats: EXE6's rules/save
//! (content/exe6/rules/save, rules/levels) runs the reload on the navi's
//! fresh stats as the round is set up, the game cleared and in the real
//! world, so a round starts as the save's reload left the navi. The chip
//! lab's link navi recordings replay that reload (exe6-compat starts each
//! such side from its fresh stats). An EXE5 team navi's level is its
//! story's progress, which its attacks' damage reads; its HP is the save's
//! (`story` gives what the story leaves it at a level, which a tool fills
//! in).

use nettai_battle::content::Content;
use nettai_content_api::NaviHandle;

/// Whether `navi` takes its stats from its level: a link navi's (it has
/// levels and doesn't change form), or the story's (EXE5's team navis,
/// `story`).
pub fn has_levels(content: &Content, navi: NaviHandle) -> bool {
    let data = content.navi(navi);
    data.levels.is_some() && !data.changes_form() || data.story.is_some()
}

#[cfg(test)]
mod tests {
    use crate::testing::exe6_content;
    use crate::{Match, ids};
    use nettai_battle::Content;
    use nettai_battle::rules::Fact;
    use nettai_battle::setup::NaviStats;
    use nettai_content_api::Value;
    use std::sync::Arc;

    /// A live EXE6 match whose right side operates `navi` (a link navi) at
    /// `level`, its Crosses none and its folder's Regular chip none.
    fn with_link_navi(content: &Arc<Content>, navi: &str, level: u8) -> Match {
        let mut m = crate::pick::live(content, "exe6", 3, None).unwrap();
        let s = &mut m.sides[1];
        (s.navi, s.navicust) = (ids::navi(content, "exe6", navi).unwrap(), None);
        s.set_level(content, Some(level)).unwrap();
        s.set_fact(content, "crosses", &[]).unwrap();
        s.folder.regular = None;
        m
    }

    /// The right side's stats as the round starts them.
    fn started(content: &Arc<Content>, m: &Match) -> NaviStats {
        crate::check::round_stats(content, m).unwrap()[1]
    }

    /// Each link navi at level 14 with the game cleared is what Tango's
    /// save editor writes (its table, taken from the game's own equips).
    #[test]
    fn level_14_is_tangos() {
        let content = exe6_content();
        // (navi, attack, rapid, charge, custom, Mega, max HP, base HP.)
        let tango = [
            ("heatman", 3, 2, 2, 6, 6, 2000, 800),
            ("elecman", 3, 2, 2, 6, 7, 1900, 800),
            ("slashman", 2, 4, 1, 7, 6, 1800, 800),
            ("eraseman", 3, 2, 3, 6, 7, 1500, 700),
            ("chargeman", 3, 2, 4, 8, 6, 1800, 800),
            ("spoutman", 3, 2, 2, 6, 6, 1900, 800),
            ("tomahawkman", 4, 2, 2, 6, 7, 1800, 800),
            ("tenguman", 3, 4, 1, 7, 6, 1800, 800),
            ("groundman", 4, 2, 2, 6, 7, 2000, 800),
            ("dustman", 2, 3, 2, 8, 6, 2000, 800),
            ("protoman", 4, 3, 3, 7, 6, 1400, 800),
        ];
        for (key, attack, rapid, charge, custom, mega, max_hp, base_hp) in tango {
            let s = started(&content, &with_link_navi(&content, key, 14));
            assert_eq!(
                (s.attack, s.rapid, s.charge, s.custom_level, s.mega_level, s.giga_level, s.max_hp, s.hp, s.max_base_hp),
                (attack, rapid, charge, custom, mega, 1, max_hp, max_hp, base_hp),
                "{key}"
            );
        }
        let tengu = started(&content, &with_link_navi(&content, "tenguman", 14));
        assert!(tengu.float_shoes && tengu.air_shoes);
        let back = |level| started(&content, &with_link_navi(&content, "protoman", level)).weapons.back_special.map(|w| content.defs.weapon(w).key.clone());
        assert_eq!(back(14).as_deref(), Some("protoman/back-special-2"), "the reflecting guard from level 10");
        assert_eq!(back(9).as_deref(), Some("protoman/back-special"), "the guard that only guards below");
    }

    /// The chip lab's ProtoMan at level 5 (`navis/navi-11-stepswrd/level-5`):
    /// Attack, Rapid and Charge 1, 1150 HP over a base of 800.
    #[test]
    fn protoman_at_level_5() {
        let content = exe6_content();
        let s = started(&content, &with_link_navi(&content, "protoman", 5));
        assert_eq!((s.attack, s.rapid, s.charge, s.custom_level, s.mega_level, s.max_hp, s.max_base_hp), (1, 1, 1, 5, 5, 1150, 800));
    }

    /// The reload keeps what the save brings (the Regular memory and the
    /// sun, which the side states) and is its level's over its fresh stats;
    /// the base HP is the cleared game's; the base HP the side states is
    /// MegaMan's alone.
    #[test]
    fn what_the_reload_keeps() {
        let content = exe6_content();
        let mut m = with_link_navi(&content, "heatman", 0);
        let s = &mut m.sides[1];
        s.set_fact(&content, "reg_up", &[Fact::Value(Value::Int(50))]).unwrap();
        s.set_fact(&content, "sun", &[Fact::Value(Value::Bool(true))]).unwrap();
        s.set_fact(&content, "hp", &[Fact::Value(Value::Int(1234))]).unwrap();
        let st = started(&content, &m);
        let fresh = NaviStats::fresh(m.sides[1].navi, &content).unwrap();
        assert_eq!((st.reg_up, st.sun), (50, true));
        assert_eq!((st.custom_level, st.mega_level, st.giga_level), (fresh.custom_level, fresh.mega_level, fresh.giga_level));
        assert_eq!((st.max_base_hp, st.max_hp, st.hp), (800, 900, 900));
    }

    /// What a link navi's level gives it in battle is EXE6's rules'
    /// (rules/by_level.luau), asked as the round is set up: HeatMan's chip
    /// bonus by his level (`byte_8021300`'s row), ChargeMan's charged chips
    /// from level 3 (`byte_8021369`) and his Fire charge by his level
    /// (`byte_802136D`); MegaMan's navi has none of them.
    #[test]
    fn a_link_navis_level_gives_it_its_rows() {
        let content = exe6_content();
        let given = |navi: &str, level: u8| crate::check::start(&content, &with_link_navi(&content, navi, level)).unwrap().given.navis[1];
        assert_eq!([0, 6, 7, 14].map(|l| given("heatman", l).chip_bonus), [0, 0, 30, 50]);
        assert_eq!([2, 3].map(|l| given("chargeman", l).charges), [false, true]);
        assert_eq!([0, 3, 14].map(|l| given("chargeman", l).fire_charge), [Some(0), Some(30), Some(100)]);
        let m = crate::pick::live(&content, "exe6", 3, None).unwrap();
        let b = crate::check::start(&content, &m).unwrap();
        assert_eq!(b.given.navis[0], nettai_battle::given::NaviGiven::default());
    }

    /// A level's gains clamp: MegaMan from a navi code at level 14 gets its
    /// gains over his NaviCust (EXE6's rules/navicust), the buster's levels
    /// at 4, the Mega level at 10, the custom level at 8.
    #[test]
    fn a_levels_gains_clamp() {
        let content = exe6_content();
        let mut m = crate::pick::live(&content, "exe6", 3, None).unwrap();
        m.sides[0].set_level(&content, Some(14)).unwrap();
        let s = crate::check::round_stats(&content, &m).unwrap()[0];
        let fresh = NaviStats::fresh(m.sides[0].navi, &content).unwrap();
        let g = content.navi(m.sides[0].navi).levels.as_ref().unwrap().by_level[14];
        let at = |v: u8, n: u8, most: u8| (v + n).min(most);
        assert_eq!(
            (s.attack, s.rapid, s.charge, s.mega_level, s.custom_level, s.max_hp),
            (
                at(fresh.attack, g.attack, 4),
                at(fresh.rapid, g.rapid, 4),
                at(fresh.charge, g.charge, 4),
                at(fresh.mega_level, g.mega_level, 10),
                at(fresh.custom_level, g.custom_level, 8),
                100 + g.hp
            )
        );
    }

    /// A match file names a link navi and its level, and no stats: its
    /// round's are the level's, with the Regular memory the file states.
    #[test]
    fn a_match_file_gives_a_link_navi_its_levels_stats() {
        let content = exe6_content();
        let mut m = with_link_navi(&content, "heatman", 14);
        m.sides[1].set_fact(&content, "reg_up", &[Fact::Value(Value::Int(50))]).unwrap();
        let text = crate::write(&content, &m);
        let right = &text[text.find("[right]").unwrap()..];
        assert!(right.contains("level = 14") && right.contains("reg_up = 50") && !text.contains("stats"), "{text}");
        let back = crate::parse(&content, &text).unwrap();
        assert_eq!(back, m);
        let s = started(&content, &back);
        assert_eq!((s.max_hp, s.attack, s.reg_up), (2000, 3, 50));
    }
}
