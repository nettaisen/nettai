//! A link navi's stats as its save gives them at its level: BN6's reload
//! (`reloadCurNaviBaseStats_8120df0`, with the HP as
//! `reloadCurNaviStatBoosts_813c3ac` leaves it), which the PET runs when a
//! navi code is received or the navi switched, before any battle. Its
//! tables are the navi definitions' `levels` (content/bn6/navis/*/navi.luau);
//! docs/engine/link-navis.md has the routines and how the level is set.
//!
//! Tools fill a side's stats from it: a match file's stats block is what
//! differs from the navi's stats at its level ([`Side::save_base`]), and
//! the editor fills them in as the level or the navi changes. The
//! simulation never runs it: a round's stats are the save's, which already
//! carry the level's (a recording's do). That is also why it is not a BN6
//! system's hook: the rules framework calls its systems inside a battle,
//! on a round's setup, whose stats a `round_setup` hook would raise a
//! second time, while a match needs the stats before any battle exists.

use crate::Side;
use nettai_battle::content::Content;
use nettai_battle::custom::GameVersion;
use nettai_battle::setup::NaviStats;
use nettai_content_api::NaviHandle;

/// What the reload is told besides the navi: its link navi level (none:
/// the save's link navi flag, event 0x163, clear), the story's progress
/// (`sub_8121108`: the highest of event flags 0x400, 0x500, 0x600, 0x800,
/// 0xA00, 0xC00 and 0xE00 set, 0 to 6), and whether the PET is in the real
/// world (map groups below 0x80), where the HP is set to its maximum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reload {
    pub level: Option<u8>,
    pub progress: usize,
    pub real_world: bool,
}

/// The story's progress a match assumes: the game cleared, every flag set
/// (as the chip lab's saves and Tango's have it).
pub const CLEARED: usize = 6;

impl Reload {
    /// As a match has it: at `level`, the game cleared, in the real world.
    pub fn at(level: Option<u8>) -> Reload {
        Reload { level, progress: CLEARED, real_world: true }
    }
}

/// `reloadCurNaviBaseStats_8120df0` for `navi`, a navi that doesn't change
/// form (a link navi), over `from` (the stats block the save had: the
/// navi's own when its level changes, the other navi's when it is switched
/// to), then the HP as `reloadCurNaviStatBoosts_813c3ac` leaves it:
///
/// 1. its fresh stats (`init_8013B4E`: [`NaviStats::fresh`]);
/// 2. what the save keeps from `from` (`byte_81210C8`): the folder, its
///    Regular and tag chips, the Regular memory and the HP; without a level,
///    the custom, Mega and Giga levels too;
/// 3. the base and maximum HP by the story's progress (`off_8120F44`);
/// 4. with a level, its gains ([`add_level`]);
/// 5. in the real world, the HP its maximum; else the HP kept, but not
///    past it.
///
/// None for a navi without levels or fresh stats, for the navi that changes
/// form (MegaMan: his stats are his NaviCust's, and a level of his adds to
/// them after it, [`add_level`]; docs/engine/link-navis.md), and for a
/// level or progress past the navi's tables.
pub fn reloaded(content: &Content, navi: NaviHandle, from: &NaviStats, r: Reload) -> Option<NaviStats> {
    let data = content.navi(navi);
    if data.changes_form() {
        return None;
    }
    let levels = data.levels.as_ref()?;
    let base = *levels.base_hp.get(r.progress)?;
    let mut s = NaviStats::fresh(navi, content)?;
    // What the save keeps (the folder's third Regular and tag chips, +0x30
    // and +0x5A, aren't modeled).
    s.folder = from.folder;
    s.folder_reg = from.folder_reg;
    s.reg_up = from.reg_up;
    s.folder_tags = from.folder_tags;
    s.hp = from.hp;
    if r.level.is_none() {
        s.mega_level = from.mega_level;
        s.giga_level = from.giga_level;
        s.custom_level = from.custom_level;
    }
    s.max_hp = base;
    s.max_base_hp = base;
    if let Some(level) = r.level {
        add_level(content, navi, level, &mut s)?;
    }
    s.hp = if r.real_world { s.max_hp } else { s.hp.min(s.max_hp) };
    Some(s)
}

/// `sub_8121154` with `sub_8123208`: what level `level` adds to `stats`
/// (the navi's `levels.by_level`): its HP to the maximum (not the base
/// HP); the buster's Attack, Rapid and Charge, to 4 at most, the Mega
/// level, to 10, and the custom level, to 8 (each clamped even when the
/// level adds nothing); SuperArmor, FloatShoes and AirShoes; the B+Back
/// special it names. None for a navi without levels or a level past them.
pub fn add_level(content: &Content, navi: NaviHandle, level: u8, stats: &mut NaviStats) -> Option<()> {
    let g = *content.navi(navi).levels.as_ref()?.by_level.get(level as usize)?;
    let clamp = |v: u8, n: u8, most: u8| (v as u16 + n as u16).min(most as u16) as u8;
    stats.max_hp = stats.max_hp.wrapping_add(g.hp);
    stats.attack = clamp(stats.attack, g.attack, 4);
    stats.rapid = clamp(stats.rapid, g.rapid, 4);
    stats.charge = clamp(stats.charge, g.charge, 4);
    stats.mega_level = clamp(stats.mega_level, g.mega_level, 10);
    stats.custom_level = clamp(stats.custom_level, g.custom_level, 8);
    stats.air_shoes |= g.air_shoes;
    stats.float_shoes |= g.float_shoes;
    stats.super_armor |= g.super_armor;
    if let Some(w) = g.back_special {
        stats.weapons.back_special = Some(w);
    }
    Some(())
}

/// Whether `navi` takes its stats from a link navi level (it has levels
/// and doesn't change form).
pub fn has_levels(content: &Content, navi: NaviHandle) -> bool {
    let data = content.navi(navi);
    data.levels.is_some() && !data.changes_form()
}

impl Side {
    /// `navi`'s stats as a save gives them, of `game`, as a battle starts
    /// them: a link navi's at `level` (its reload over its fresh stats),
    /// else its fresh stats ([`Side::base_stats`]). What a match file's
    /// stats block is written over.
    pub fn save_base(content: &Content, navi: NaviHandle, game: GameVersion, level: u8) -> NaviStats {
        let fresh = Side::base_stats(content, navi, game);
        match reloaded(content, navi, &fresh, Reload::at(Some(level))) {
            Some(s) => crate::starting(content, s, game),
            None => fresh,
        }
    }

    /// The side's stats as its save's reload gives them at its level, over
    /// its own (what the save keeps carried), as a battle starts them: what
    /// the editor fills in as the level changes, and what it shows an
    /// edited stat against. None for a navi without levels.
    pub fn reloaded(&self, content: &Content) -> Option<NaviStats> {
        self.reloaded_as(content, self.navi)
    }

    /// The side's stats were it to switch to `navi` (a link navi), at its
    /// level: the new navi's reload over the side's stats (`from`: what the
    /// save keeps carries over, as the game's switch carries it). None for
    /// a navi without levels.
    pub fn reloaded_as(&self, content: &Content, navi: NaviHandle) -> Option<NaviStats> {
        reloaded(content, navi, &self.stats, Reload::at(Some(self.navi_level))).map(|s| crate::starting(content, s, self.game))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::bn6_content;

    fn navi(content: &Content, key: &str) -> NaviHandle {
        content.defs.navi_by_key(key).unwrap()
    }

    /// Each link navi at level 14 with the game cleared is what Tango's
    /// save editor writes (its table, taken from the game's own equips).
    #[test]
    fn level_14_is_tangos() {
        let content = bn6_content();
        // (navi, attack, rapid, charge, custom, Mega, max HP, base HP.)
        let tango = [
            ("bn6:heatman", 3, 2, 2, 6, 6, 2000, 800),
            ("bn6:elecman", 3, 2, 2, 6, 7, 1900, 800),
            ("bn6:slashman", 2, 4, 1, 7, 6, 1800, 800),
            ("bn6:eraseman", 3, 2, 3, 6, 7, 1500, 700),
            ("bn6:chargeman", 3, 2, 4, 8, 6, 1800, 800),
            ("bn6:spoutman", 3, 2, 2, 6, 6, 1900, 800),
            ("bn6:tomahawkman", 4, 2, 2, 6, 7, 1800, 800),
            ("bn6:tenguman", 3, 4, 1, 7, 6, 1800, 800),
            ("bn6:groundman", 4, 2, 2, 6, 7, 2000, 800),
            ("bn6:dustman", 2, 3, 2, 8, 6, 2000, 800),
            ("bn6:protoman", 4, 3, 3, 7, 6, 1400, 800),
        ];
        for (key, attack, rapid, charge, custom, mega, max_hp, base_hp) in tango {
            let n = navi(&content, key);
            let s = Side::save_base(&content, n, GameVersion::Falzar, 14);
            assert_eq!(
                (s.attack, s.rapid, s.charge, s.custom_level, s.mega_level, s.giga_level, s.max_hp, s.hp, s.max_base_hp),
                (attack, rapid, charge, custom, mega, 1, max_hp, max_hp, base_hp),
                "{key}"
            );
        }
        let tengu = Side::save_base(&content, navi(&content, "bn6:tenguman"), GameVersion::Falzar, 14);
        assert!(tengu.float_shoes && tengu.air_shoes);
        let protoman = navi(&content, "bn6:protoman");
        let back = |level| Side::save_base(&content, protoman, GameVersion::Falzar, level).weapons.back_special.map(|w| content.defs.weapon(w).key.clone());
        assert_eq!(back(14).as_deref(), Some("bn6:protoman/back-special-2"), "the reflecting guard from level 10");
        assert_eq!(back(9).as_deref(), Some("bn6:protoman/back-special"), "the guard that only guards below");
    }

    /// The chip lab's ProtoMan at level 5 (`navis/navi-11-stepswrd/level-5`):
    /// Attack, Rapid and Charge 1, 1150 HP over a base of 800.
    #[test]
    fn protoman_at_level_5() {
        let content = bn6_content();
        let s = Side::save_base(&content, navi(&content, "bn6:protoman"), GameVersion::Falzar, 5);
        assert_eq!((s.attack, s.rapid, s.charge, s.custom_level, s.mega_level, s.max_hp, s.max_base_hp), (1, 1, 1, 5, 5, 1150, 800));
    }

    /// The reload keeps the save's folder fields and Regular memory, and
    /// without a level the custom, Mega and Giga levels too; the story's
    /// progress gives the base HP; in the internet the HP is kept, up to
    /// the maximum.
    #[test]
    fn what_the_reload_keeps() {
        let content = bn6_content();
        let heatman = navi(&content, "bn6:heatman");
        let mut from = NaviStats::fresh(heatman, &content).unwrap();
        (from.reg_up, from.folder, from.folder_reg, from.hp) = (50, 2, [3, 0xFF], 2500);
        (from.custom_level, from.mega_level, from.giga_level, from.attack) = (7, 8, 3, 4);
        let at = |from: &NaviStats, level, progress, real_world| reloaded(&content, heatman, from, Reload { level, progress, real_world }).unwrap();
        let s = at(&from, Some(0), CLEARED, true);
        assert_eq!((s.reg_up, s.folder, s.folder_reg), (50, 2, [3, 0xFF]));
        assert_eq!((s.custom_level, s.mega_level, s.giga_level, s.attack), (5, 5, 1, 0), "the level's, over fresh");
        assert_eq!((s.max_base_hp, s.max_hp, s.hp), (800, 900, 900));
        let s = at(&from, None, 0, true);
        assert_eq!((s.custom_level, s.mega_level, s.giga_level, s.attack), (7, 8, 3, 0), "no level: the save's");
        assert_eq!((s.max_base_hp, s.max_hp, s.hp), (300, 300, 300));
        let s = at(&from, Some(14), 3, false);
        assert_eq!((s.max_base_hp, s.max_hp, s.hp), (400, 1600, 1600), "the internet: the save's HP, up to the maximum");
        from.hp = 10;
        assert_eq!(at(&from, Some(14), 3, false).hp, 10);
        assert!(reloaded(&content, heatman, &from, Reload::at(Some(15))).is_none(), "past the levels");
        assert!(reloaded(&content, navi(&content, "bn6:megaman"), &from, Reload::at(Some(3))).is_none(), "MegaMan's are his NaviCust's");
    }

    /// A level's gains clamp: the buster's levels at 4, the Mega level at
    /// 10, the custom level at 8, even when the level adds nothing.
    #[test]
    fn a_levels_gains_clamp() {
        let content = bn6_content();
        let megaman = navi(&content, "bn6:megaman");
        let mut s = NaviStats::fresh(megaman, &content).unwrap();
        (s.attack, s.rapid, s.charge, s.mega_level, s.custom_level, s.max_hp) = (3, 9, 4, 9, 8, 1000);
        add_level(&content, megaman, 14, &mut s).unwrap();
        assert_eq!((s.attack, s.rapid, s.charge, s.mega_level, s.custom_level, s.max_hp), (4, 4, 4, 10, 8, 1300));
        assert!(add_level(&content, megaman, 15, &mut s).is_none());
    }

    /// A match file names a link navi and its level: its stats are the
    /// level's, so the file's stats block has only what the save keeps (the
    /// Regular memory MegaMan had, carried over by the switch) and an edited
    /// stat.
    #[test]
    fn a_match_file_gives_a_link_navi_its_levels_stats() {
        let content = bn6_content();
        let mut m = crate::draw::live(&content, 3, None).unwrap();
        let heatman = navi(&content, "bn6:heatman");
        let s = &mut m.sides[1];
        s.navi_level = 14;
        s.stats = s.reloaded_as(&content, heatman).unwrap();
        (s.navi, s.crosses) = (heatman, None);
        let text = crate::write(&content, &m);
        assert!(text.contains("level = 14") && text.ends_with("[right.stats]\nregular_memory = 50\n"), "{text}");
        let back = crate::parse(&content, &text).unwrap();
        assert_eq!((back.sides[1].stats.max_hp, back.sides[1].stats.attack), (2000, 3));
        assert_eq!(back, m);
        m.sides[1].stats.attack = 1;
        let text = crate::write(&content, &m);
        assert!(text.ends_with("[right.stats]\nattack = 1\nregular_memory = 50\n"), "{text}");
        assert_eq!(crate::parse(&content, &text).unwrap(), m);
    }
}
