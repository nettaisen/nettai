//! A navi's stats as the story leaves them at its level: EXE5's team navis
//! (the navi definitions' `story`, content/exe5/navis/*/init.luau). The
//! game's routine sets a team navi's HP (current, maximum and base) from a
//! table by the story's progress, at a new game's start and from a map
//! script as the story moves on; the level a battle reads (a team navi's
//! damage rows) is that progress up to the last level. A level below the
//! last is the progress itself; at the last the story is taken as done.
//!
//! Tools fill a side's stats from it, as they do a link navi's from its
//! reload (`link_navis`); the simulation never runs it.

use nettai_battle::content::Content;
use nettai_battle::setup::NaviStats;
use nettai_content_api::NaviHandle;

/// `navi`'s stats at `level` over `from` (the stats the side had): its
/// fresh stats with what the save keeps from `from` (the folder, its
/// Regular and tag chips and the Regular memory) and the story's HP. None
/// for a navi without a story or fresh stats, without a level, and for a
/// level past its last.
pub fn at(content: &Content, navi: NaviHandle, from: &NaviStats, level: Option<u8>) -> Option<NaviStats> {
    let hp = content.navi(navi).story.as_ref()?.hp_at(level?)?;
    let mut s = NaviStats::fresh(navi, content)?;
    s.folder = from.folder;
    s.folder_reg = from.folder_reg;
    s.reg_up = from.reg_up;
    s.folder_tags = from.folder_tags;
    (s.hp, s.max_hp, s.max_base_hp) = (hp, hp, hp);
    Some(s)
}

/// The last level of `navi`'s story (none: it has none).
pub fn max_level(content: &Content, navi: NaviHandle) -> Option<u8> {
    content.navi(navi).story.as_ref().map(|s| s.max_level)
}

#[cfg(test)]
mod tests {
    use crate::Side;
    use crate::testing::exe5_content;

    /// ProtoMan's HP by his level: the progress below the last level, the
    /// story done at the last; MegaMan has no story.
    #[test]
    fn a_team_navis_hp_follows_its_level() {
        let content = exe5_content();
        let navi = |name: &str| crate::ids::navi(&content, "exe5", name).unwrap();
        let protoman = navi("protoman");
        let hp = |level| {
            let s = Side::save_base(&content, protoman, None, Some(level));
            assert_eq!((s.hp, s.max_base_hp), (s.max_hp, s.max_hp));
            s.max_hp
        };
        assert_eq!([hp(0), hp(1), hp(5), hp(6)], [200, 300, 500, 800]);
        assert_eq!(super::max_level(&content, protoman), Some(6));
        // Past the last level: the fresh stats (the checks refuse the level).
        assert_eq!(hp(7), 200);
        assert_eq!(super::max_level(&content, navi("megaman")), None);
    }

    /// A match whose left side operates ProtoMan at level 3: its checks
    /// pass, its file names the navi and the level and no stats, and its
    /// round starts with his stats and level; a level past his last, or
    /// none, is refused.
    #[test]
    fn a_match_with_a_team_navi() {
        let content = exe5_content();
        let protoman = crate::ids::navi(&content, "exe5", "protoman").unwrap();
        let mut m = crate::pick::live(&content, "exe5", 3, None).unwrap();
        let s = &mut m.sides[0];
        s.navi_level = Some(3);
        s.stats = s.reloaded_as(&content, protoman).unwrap();
        (s.navi, s.navicust) = (protoman, None);
        assert_eq!(crate::check::check_match(&content, &m), Vec::<String>::new());
        let text = crate::write(&content, &m);
        assert!(text.contains("navi = \"protoman\"") && text.contains("level = 3") && !text.contains("[left.stats]"), "{text}");
        assert_eq!(crate::parse(&content, &text).unwrap(), m);
        let b = crate::check::start(&content, &m).unwrap();
        assert_eq!((b.stats[0].max_hp, b.stats[0].hp, b.navi_levels[0]), (400, 400, 3));
        assert_eq!(b.stats[0].weapons.back_special_damage, 50);
        // An edited HP stays, in the file's stats block.
        m.sides[0].stats.hp = 250;
        let text = crate::write(&content, &m);
        assert!(text.contains("[left.stats]\ncurrent_hp = 250\n"), "{text}");
        assert_eq!(crate::parse(&content, &text).unwrap(), m);
        m.sides[0].navi_level = Some(7);
        let problems = crate::check::check_match(&content, &m);
        assert!(problems.iter().any(|p| p.contains("level 7") && p.contains("0 to 6")), "{problems:?}");
        m.sides[0].navi_level = None;
        let problems = crate::check::check_match(&content, &m);
        assert!(problems.iter().any(|p| p.contains("has no level")), "{problems:?}");
    }
}
