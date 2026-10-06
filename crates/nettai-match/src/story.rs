//! What the story leaves EXE5's team navis at a level (the navi
//! definitions' `story`, content/exe5/navis/*/init.luau). The game's routine
//! sets a team navi's HP (current, maximum and base) from a table by the
//! story's progress, at a new game's start and from a map script as the
//! story moves on; the level a battle reads (a team navi's damage rows) is
//! that progress up to the last level. A level below the last is the
//! progress itself; at the last the story is taken as done.
//!
//! A side states its team navi's level, and no HP: EXE5's rules/save
//! gives the navi the story's HP at its level as the round is set up
//! (content/exe5/rules/save). [`hp_at`] says what that is, for a tool.

use nettai_battle::content::Content;
use nettai_content_api::NaviHandle;

/// The HP the story leaves `navi` at `level`; none for a navi without a
/// story, and for a level past its last.
pub fn hp_at(content: &Content, navi: NaviHandle, level: u8) -> Option<u16> {
    content.navi(navi).story.as_ref()?.hp_at(level)
}

/// The last level of `navi`'s story (none: it has none).
pub fn max_level(content: &Content, navi: NaviHandle) -> Option<u8> {
    content.navi(navi).story.as_ref().map(|s| s.max_level)
}

#[cfg(test)]
mod tests {
    use crate::testing::exe5_content;

    /// ProtoMan's HP by his level: the progress below the last level, the
    /// story done at the last; MegaMan has no story.
    #[test]
    fn a_team_navis_hp_follows_its_level() {
        let content = exe5_content();
        let navi = |name: &str| crate::ids::navi(&content, "exe5", name).unwrap();
        let protoman = navi("protoman");
        let hp = |level| super::hp_at(&content, protoman, level);
        assert_eq!([hp(0), hp(1), hp(5), hp(6), hp(7)], [Some(200), Some(300), Some(500), Some(800), None]);
        assert_eq!(super::max_level(&content, protoman), Some(6));
        assert_eq!(super::max_level(&content, navi("megaman")), None);
    }

    /// A match whose left side operates ProtoMan: he is at his story's last
    /// level where the side states none, its checks pass, its file names the
    /// navi and the level and no HP, and its round starts with the story's HP
    /// at the level; an HP of the side's own, a level past his last, or
    /// none, is refused.
    #[test]
    fn a_match_with_a_team_navi() {
        let content = exe5_content();
        let protoman = crate::ids::navi(&content, "exe5", "protoman").unwrap();
        let mut m = crate::pick::live(&content, "exe5", 3, None).unwrap();
        let s = &mut m.sides[0];
        s.set_navi(&content, protoman).unwrap();
        assert_eq!(s.level(&content), Some(6), "his story's last");
        assert_eq!(crate::check::check_match(&content, &m), Vec::<String>::new());
        let text = crate::write(&content, &m);
        assert!(text.contains("navi = \"protoman\"") && text.contains("level = 6") && !text.contains("hp ="), "{text}");
        assert_eq!(crate::parse(&content, &text).unwrap(), m);
        let b = crate::check::start(&content, &m).unwrap();
        // (The HP the round starts with is the maximum: a link battle's
        // start, `init_hp`.)
        assert_eq!((b.stats[0].max_base_hp, b.stats[0].max_hp), (800, 800));
        assert_eq!(Some(b.stats[0].max_hp), super::hp_at(&content, protoman, 6));
        // (At level 3: his own chip's damage, StepSwrd's, is his row's at
        // his level, what EXE5's rules gave his side as the round was set
        // up; the other side, MegaMan, has no level and none of it.)
        let mut three = m.clone();
        three.sides[0].set_level(&content, Some(3)).unwrap();
        let b = crate::check::start(&content, &three).unwrap();
        let stepswrd = crate::ids::chip(&content, "exe5", "stepswrd").unwrap();
        assert_eq!((b.given.damage(stepswrd, 0), b.given.damage(stepswrd, 1)), (Some(100), Some(0)));
        assert_eq!(b.stats[0].weapons.back_special_damage, 50);
        assert_eq!(Some(b.stats[0].max_hp), super::hp_at(&content, protoman, 3));
        // An HP of the side's own has no effect, and is refused.
        let mut own = m.clone();
        own.sides[0].set_fact(&content, "hp", &[nettai_battle::rules::Fact::Value(nettai_content_api::Value::Int(250))]).unwrap();
        let problems = crate::check::check_match(&content, &own);
        assert!(problems.iter().any(|p| p.contains("hp: ProtoMan's HP is its level's")), "{problems:?}");
        m.sides[0].set_level(&content, Some(7)).unwrap();
        let problems = crate::check::check_match(&content, &m);
        assert!(problems.iter().any(|p| p.contains("level 7") && p.contains("0 to 6")), "{problems:?}");
        m.sides[0].set_level(&content, None).unwrap();
        let problems = crate::check::check_match(&content, &m);
        assert!(problems.iter().any(|p| p.contains("has no level")), "{problems:?}");
    }
}
