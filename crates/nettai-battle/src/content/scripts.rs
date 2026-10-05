//! The content's scripts: the Luau modules of the packs a load reads
//! (docs/design/content-model-v2.md §4.0), each named by its pack and its
//! path in it (`exe6:chips/minibomb/init`, `exelib:swords/slash`), which
//! live next to what they define.
//!
//! Each pack has a manifest (`manifest.toml`: its name, its kind and the
//! support packs it depends on), and a game pack a top module
//! (`<game>/init.luau`) that requires what the game has. The define phase
//! runs each game's top module, and each `require` finds and reads its
//! module as it is reached (`packs::find`): from the modules held in
//! memory, else from the packs' folders ([`Scripts::dirs`]). What the load
//! read is the content's modules from then on: its hash covers them, and a
//! runtime loads them again from memory, never from a folder. A module
//! requires only its own pack's modules and those of the support packs its
//! pack depends on. Every id a module writes is local to its game
//! (`minibomb`).
//!
//! [`Content::define`](super::Content::define) turns what the modules
//! define into what the script runtime binds (`content::defs`). Nothing in
//! the engine says which kind, action or hook is a script: whatever the
//! content defines runs as content, and the engine's own Rust runs the rest.

use std::collections::BTreeMap;

use nettai_content_api::{PackKind, PackManifest, keys, packs};

/// The content's Luau modules.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Scripts {
    /// Source text by module name: its pack, then its path in the pack
    /// without `.luau` (`exe6:objects/sun-beam/sun_beam`, `keys::module_name`).
    /// Before the define phase, the modules held in memory (a test's, a
    /// tool's stand-ins); after it, those and every module the load read.
    pub modules: BTreeMap<String, String>,
    /// The packs the content loads (their manifests), in load order: the
    /// support packs, then the games. None: every module loads (modules a
    /// test makes alone).
    pub packs: Vec<PackManifest>,
    /// Where the define phase reads a module `modules` hasn't: the packs'
    /// folders (content/'s). No part of what the content is.
    pub dirs: ModuleDirs,
    /// Their bytecode, as the define phase compiled it (a runtime then
    /// skips the compiler).
    pub compiled: CompiledModules,
}

impl Scripts {
    /// The modules under content/'s directory `dir`, by path in it, without
    /// bytecode (tests and tools that hold modules in memory).
    pub fn dir(dir: &str, modules: BTreeMap<String, String>) -> Scripts {
        let mut s = Scripts::default();
        s.add_dir(dir, modules);
        s
    }

    /// Add the modules under content/'s directory `dir`, by path in it.
    pub fn add_dir(&mut self, dir: &str, modules: BTreeMap<String, String>) {
        for (path, source) in modules {
            self.modules.insert(Scripts::name(dir, &path), source);
        }
    }

    /// The game packs the content loads, in order.
    pub fn games(&self) -> Vec<String> {
        self.packs.iter().filter(|p| p.kind == PackKind::Game).map(|p| p.id.clone()).collect()
    }

    /// The manifest of pack `id`, if the content loads it.
    pub fn manifest(&self, id: &str) -> Option<&PackManifest> {
        self.packs.iter().find(|p| p.id == id)
    }

    /// Game `game`'s modules (by path in its directory), with a manifest
    /// that depends on every support pack the content holds; without a top
    /// module among them (`init`), one that requires every one of them
    /// ([`Scripts::init_for`]). (Tests and tools that hold a game's modules
    /// in memory; content/'s packs have their own.)
    pub fn add_game(&mut self, game: &str, mut modules: BTreeMap<String, String>) {
        let depends = self.packs.iter().filter(|p| p.kind == PackKind::Support).map(|p| p.id.clone()).collect();
        if !modules.contains_key(packs::INIT) {
            let init = Scripts::init_for(&modules);
            modules.insert(packs::INIT.to_string(), init);
        }
        self.add_dir(game, modules);
        self.set_manifest(PackManifest { id: game.to_string(), kind: PackKind::Game, depends, extract: None });
    }

    /// Support pack `id` in folder `dir` (content/exelib): its manifest,
    /// which the games the content holds depend on, and its modules read
    /// from the folder as a load requires them.
    pub fn add_support_dir(&mut self, id: &str, dir: impl Into<std::path::PathBuf>) {
        self.read_from(id, dir);
        self.add_support(id, BTreeMap::new());
    }

    /// Pack `id`'s modules are read from folder `dir` (content/<id>) as a
    /// load requires them: each one memory holds none of the name for.
    pub fn read_from(&mut self, id: &str, dir: impl Into<std::path::PathBuf>) {
        self.dirs.0.0.insert(id.to_string(), dir.into());
    }

    /// Support pack `id`'s modules (by path in its directory), with its
    /// manifest; the games the content holds depend on it.
    pub fn add_support(&mut self, id: &str, modules: BTreeMap<String, String>) {
        self.add_dir(id, modules);
        for p in self.packs.iter_mut().filter(|p| p.kind == PackKind::Game) {
            if !p.depends.iter().any(|u| u == id) {
                p.depends.push(id.to_string());
            }
        }
        self.set_manifest(PackManifest { id: id.to_string(), kind: PackKind::Support, ..Default::default() });
    }

    /// Pack `manifest`'s, in place of one of its name (support packs before
    /// the games).
    pub fn set_manifest(&mut self, manifest: PackManifest) {
        self.packs.retain(|p| p.id != manifest.id);
        let at = if manifest.kind == PackKind::Support {
            self.packs.iter().position(|p| p.kind == PackKind::Game).unwrap_or(self.packs.len())
        } else {
            self.packs.len()
        };
        self.packs.insert(at, manifest);
    }

    /// A top module (`init.luau`'s text) for a game whose modules are
    /// `modules` (by path in its directory): it requires every one, so
    /// every one loads, in path order, a folder by its name.
    pub fn init_for(modules: &BTreeMap<String, String>) -> String {
        let lines: String =
            modules.keys().filter(|p| p.as_str() != packs::INIT).map(|p| format!("require(\"@self/{}\")\n", keys::listed_as(p))).collect();
        format!("{GENERATED}{lines}")
    }

    /// The module name of path `path` in content/'s directory `dir`.
    pub fn name(dir: &str, path: &str) -> String {
        format!("{dir}{}{path}", keys::SEPARATOR)
    }

    /// Module `path` of content/'s directory `dir`, to change (tests and
    /// tools).
    pub fn module_mut(&mut self, dir: &str, path: &str) -> Option<&mut String> {
        self.modules.get_mut(&Scripts::name(dir, path))
    }

    /// What is wrong with the packs: a bad manifest, a pack twice, a game
    /// pack depended on or a cycle, a game without its top module.
    pub fn check_packs(&self) -> Result<(), String> {
        let mut all = BTreeMap::new();
        for p in &self.packs {
            let at = format!("{}/{}", p.id, packs::MANIFEST);
            if !keys::valid_root_name(&p.id) {
                return Err(format!("{at}: pack name {:?} is not lowercase words in - (and not \"engine\")", p.id));
            }
            if all.insert(p.id.clone(), p.clone()).is_some() {
                return Err(format!("{at}: pack {} is loaded twice", p.id));
            }
        }
        packs::load_order(&all, &self.games())?;
        let modules = (&self.modules, &self.dirs.0);
        for p in &self.packs {
            if let Some(m) = p.entry()
                && packs::module(&modules, &m).is_none()
            {
                return Err(format!(
                    "{}.luau: game pack {} has no top module: its {}.luau requires what the game has",
                    keys::module_path(&m),
                    p.id,
                    packs::INIT
                ));
            }
        }
        Ok(())
    }

    /// The modules as the define phase loads them: from each game's top
    /// module, each module read as it is required, from memory or, one
    /// memory hasn't, from the packs' folders.
    pub fn pack_to_define(&self) -> nettai_luau::Pack {
        let entries = self.packs.iter().filter_map(|p| p.entry()).collect::<Vec<_>>();
        if self.packs.is_empty() {
            // (A test's modules alone: every one, in name order.)
            return nettai_luau::Pack::new(self.modules.clone());
        }
        nettai_luau::Pack::of((self.modules.clone(), self.dirs.0.clone()), entries).with_packs(self.packs.iter().cloned())
    }

    /// The modules as a runtime loads them, with the bytecode compiled
    /// from them: from each game's top module, from memory alone (what the
    /// define phase read: no folder is read again).
    pub fn pack(&self) -> nettai_luau::Pack {
        let entries = self.packs.iter().filter_map(|p| p.entry()).collect::<Vec<_>>();
        let pack = nettai_luau::Pack::new(self.modules.clone()).with_packs(self.packs.iter().cloned()).with_compiled(self.compiled.0.clone());
        if self.packs.is_empty() { pack } else { pack.with_entries(entries) }
    }

    /// Every module the content could load, by name: those in memory and
    /// every file of the packs' folders (a tool's list: the test content's
    /// made-up assets; never what a load reads).
    pub fn available(&self) -> BTreeMap<String, String> {
        use nettai_content_api::packs::Modules;
        let mut out: BTreeMap<String, String> =
            self.dirs.0.names().into_iter().filter_map(|name| Some((name.clone(), self.dirs.0.read(&name)?))).collect();
        out.extend(self.modules.iter().map(|(k, v)| (k.clone(), v.clone())));
        out
    }
}

/// The first line of an init [`Scripts::init_for`] makes.
pub const GENERATED: &str = "-- (Made for a game held in memory: it requires every module there is.)\n";

/// The packs' folders the define phase reads modules from
/// ([`Scripts::dirs`]): where the content came from on this machine, so
/// content equality and the content hash leave it out.
#[derive(Clone, Debug, Default)]
pub struct ModuleDirs(pub packs::Dirs);

impl PartialEq for ModuleDirs {
    fn eq(&self, _: &ModuleDirs) -> bool {
        true
    }
}

impl Eq for ModuleDirs {}

impl std::hash::Hash for ModuleDirs {
    fn hash<H: std::hash::Hasher>(&self, _: &mut H) {}
}

/// Bytecode compiled from [`Scripts::modules`]: derived from them (each
/// module's with the source it was compiled from, which loading checks),
/// so content equality and the content hash leave it out.
#[derive(Clone, Default)]
pub struct CompiledModules(pub nettai_luau::Compiled);

impl std::fmt::Debug for CompiledModules {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "CompiledModules({})", self.0.len())
    }
}

impl PartialEq for CompiledModules {
    fn eq(&self, _: &CompiledModules) -> bool {
        true
    }
}

impl Eq for CompiledModules {}

impl std::hash::Hash for CompiledModules {
    fn hash<H: std::hash::Hasher>(&self, _: &mut H) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Content;

    type Game = (String, BTreeMap<String, String>);

    /// A content whose rules are stated as Rust tables (the test content's:
    /// `testing::rules`), for a test whose small ruleset names its systems
    /// alone: a ruleset states every rule, or the content's tables do.
    fn stated() -> Content {
        let mut rules = crate::content::testing::rules();
        // (Its fresh stats' weapon is a handle of the test content's.)
        rules.fresh_stats.mode9_a = None;
        Content { base_rules: Some(rules), ..Default::default() }
    }

    fn folder(name: &str, modules: &[(&str, &str)]) -> Game {
        (name.to_string(), modules.iter().map(|(p, s)| (p.to_string(), s.to_string())).collect())
    }

    /// Content of these game packs (each with a manifest that loads all its
    /// modules), defined.
    fn content(games: Vec<Game>) -> Result<Content, String> {
        let mut c = stated();
        for (name, modules) in games {
            c.scripts.add_game(&name, modules);
        }
        c.define().map_err(|e| e.message)?;
        Ok(c)
    }

    const GAME: &[(&str, &str)] = &[
        ("rules/turns", "return define.system { id = 'turns', state = { n = 'u8' } }"),
        ("rules/ruleset", "return define.ruleset { systems = { require('./turns') } }"),
        ("cards", "return define.record('card', { power = 1 })"),
        ("chips/cannon", "return define.record('chip-ish', { power = 3 })"),
    ];

    /// docs/design/content-model-v2.md §4.0: content holds one game pack
    /// (a match plays one game), every id written in full, its game first;
    /// one game pack requires nothing of another's.
    #[test]
    fn content_holds_one_game() {
        let mix: &[(&str, &str)] = &[
            ("rules/extra", "return define.system { id = 'extra' }"),
            ("rules/ruleset", "return define.ruleset { systems = { require('./extra') } }"),
            ("cards", "return define.record('card', { power = 2 })"),
        ];
        let c = content(vec![folder("game", GAME)]).unwrap();
        let d = &c.defs;
        assert_eq!((c.game(), d.game.as_str()), ("game", "game"));
        let keys: Vec<&str> = d.systems.iter().map(|s| s.key.as_str()).collect();
        assert_eq!(keys, ["turns"]);
        assert_eq!(d.ruleset().map(|r| r.systems.len()), Some(1), "the game's one ruleset");
        // Lookups are exact, of the one game's.
        assert!(d.record("cards#1").is_some());
        assert_eq!(d.record("game:cards#1"), None);
        // Two game packs are two contents.
        let e = content(vec![folder("mix", mix), folder("game", GAME)]).unwrap_err();
        assert!(e.contains("content holds one game, and these are mix and game"), "{e}");
        // One game pack requires nothing of another's.
        let mut c = stated();
        c.scripts.add_dir("game", GAME.iter().map(|(p, s)| (p.to_string(), s.to_string())).collect());
        c.scripts.set_manifest(PackManifest { id: "game".into(), kind: PackKind::Game, ..Default::default() });
        c.scripts.add_game("mix", [("rules/ruleset".to_string(), "return define.ruleset { systems = { require('@game/rules/turns') } }".to_string())].into());
        c.scripts.packs.retain(|p| p.id == "mix");
        let e = c.define().unwrap_err().message;
        assert!(e.contains("mix/rules/ruleset.luau: require(\"@game/rules/turns\"): game is no pack") || e.contains("game is a game pack"), "{e}");
    }

    #[test]
    fn what_the_namespace_refuses() {
        let e = content(vec![folder("game", &[("rules/turns", "return define.system { id = 'game:turns' }")])]).unwrap_err();
        assert!(e.contains("\"game:turns\" is not a valid id"), "{e}");
        // A section is the ruleset's field by the engine's name.
        let e = content(vec![folder("game", &[("rules/x", "return define.ruleset { Pools = { actor = 16 } }")])]).unwrap_err();
        assert!(e.contains("`Pools` is no field of a ruleset"), "{e}");
        let e = content(vec![folder("game", &[("rules/x", "return define.ruleset { pools = { actor = 0, attack = 32, effect = 32 } }")])]).unwrap_err();
        assert!(e.contains("game/rules/x.luau: ruleset: pools: a pool holds 1 to"), "{e}");
        let e = content(vec![folder("game", &[("rules/x", "return define.ruleset { pools = { actor = 'many', attack = 32, effect = 32 } }")])]).unwrap_err();
        assert!(e.contains("ruleset: pools.actor: invalid type"), "{e}");
        let e = content(vec![folder("Game", GAME)]).unwrap_err();
        assert!(e.contains("not lowercase words"), "{e}");
    }

    /// The user: "no default games anywhere please". The engine has no
    /// game's rules of its own: a ruleset states every rule that has no
    /// neutral value (`sections::REQUIRED`, and every field of them), and
    /// one that leaves a rule out is a load error naming it. Content with
    /// no ruleset has no rules.
    #[test]
    fn a_ruleset_states_every_rule() {
        const HEAD: &str = r#"
local function row(n: number, v: any): { any }
    local t = {}
    for i = 1, n do
        t[i] = v
    end
    return t
end
local none = { dx = 0, dy = 0, panels = 0 }
local any_panel = { require = 0, forbid = 0 }
local step = { grounded = { any_panel, any_panel }, floor_free = { any_panel, any_panel } }
"#;
        // The sections a ruleset states, a field a line.
        const STATED: &[(&str, &str)] = &[
            (
                "chip_use",
                r#"
        leave_on_use = true,
        anti_navi_sparkle = { dy = 0, z = 16 },
        mixed_modifiers = false,"#,
            ),
            (
                "effects",
                r#"
        shake = "battle_rng",
        spark_steps_at_start = true,
        retype = "is_and_hits",
        damage_word = "statuses_and_bug",
        obstacle_soldiers = false,
        palette_flash = "mode_runs_through_pause",
        palette_flash_order = "after_fades",
        overlays_run_while_paused = true,
        load_sets_part_palette = true,
        obstacle_actions = "own_from_6",
        full_synchro_aura = { follows_identity = true, steps_while_paused = true, stops_at_a_pause_in_the_fight = false },"#,
            ),
            (
                "flow",
                r#"
        result_words = 49,
        sequencer_before_custom = true,
        escape_check = false,
        result_wait = { normal = 100, special = 90 },
        chip_window_at_close = false,
        intro_from_black = false,
        low_hp_music = true,
        navi_win_banner = "operation_battle",
        link_backgrounds = {},"#,
            ),
            (
                "fresh_stats",
                r#"
        reg_up = 7,
        custom_level = 6,
        mood = 0x70,"#,
            ),
            (
                "panels",
                r#"
        types = { normal = { flags = 0x10 } },
        mend = { normal = 600, battle_mode_1 = 480 },
        start_visible = row(5, row(8, true)),
        front_edges = row(5, row(8, false)),
        numbers = { "missing", "broken", "normal" },
        step = step,
        dash_step = step,
        any_side_step = step,
        reservations = "unmarked","#,
            ),
            (
                "pools",
                r#"
        actor = 16,
        attack = 32,
        effect = 8,"#,
            ),
            (
                "reactions",
                r#"
        push = row(10, none),
        push_reading = "by_hitter_flip",
        hit_test = { float_shoe_needs_self_bit = true, bubbled_as_submerged = false, elec_reaches_submerged = true, guard_breaks_to = 0x1002, elec_bonus_on_sea = false },
        obstacle_slide_bounds = false,
        ice = row(6, none),
        bubble_bob = row(32, 0),
        slide_speed = { x = 0x30000, y = 0x20000 },
        overlay_restart = "reload",
        stance_counter = "next_tick",
        request_clears = { attack = { "mode9_a" }, paralysis = { "mode9_a" }, flinch = { "anti_sword_triggered" }, drag = { 0x400 } },"#,
            ),
            (
                "status",
                r#"
        hp_bug_periods = row(8, 30),
        form_tick = false,
        flash_hides_on_clear = true,
        missing_collision_status = 7,
        reactions = "flash_timer_first",
        bugs_before_drain = false,
        drain_bug_flags = true,
        no_charge_drive = false,
        hp_loss = "hp_alone",
        emotion = { mood_held = "at_zero", anger_end = "resets_mood", plain_in_battle_mode_1 = false, normal_in_a_form = true, anger_before_worn_out = false, tired_and_exhausted = true, worried_below = 40 },
        form_break = "cross_or_beast","#,
            ),
        ];
        assert_eq!(STATED.iter().map(|(name, _)| *name).collect::<Vec<_>>(), crate::content::sections::REQUIRED);
        // The ruleset, without a section or a field of one, or with a
        // field's value another.
        let ruleset = |without: Option<&str>, field: Option<(&str, &str)>, other: Option<(&str, &str)>| -> String {
            let mut out = format!("{HEAD}return define.ruleset {{\n    systems = {{}},\n");
            for (name, body) in STATED {
                if without == Some(*name) {
                    continue;
                }
                let lines: Vec<String> = body
                    .lines()
                    .filter(|l| !matches!(field, Some((s, f)) if s == *name && l.trim_start().starts_with(&format!("{f} = "))))
                    .map(|l| match other {
                        Some((from, to)) => l.replace(from, to),
                        None => l.to_string(),
                    })
                    .collect();
                out += &format!("    {name} = {{{}\n    }},\n", lines.join("\n"));
            }
            out + "}\n"
        };
        let game = |ruleset: String| -> Result<Content, String> {
            let mut c = Content::default();
            c.scripts.add_game("game", [("rules/init".to_string(), ruleset)].into());
            c.define().map_err(|e| e.message)?;
            Ok(c)
        };
        // Every rule stated: the game's rules are what it states, each a
        // choice of its own (no game has these together).
        let c = game(ruleset(None, None, None)).unwrap_or_else(|e| panic!("{e}"));
        let r = c.rules();
        use crate::content::{
            AngerEnd, DamageWordRule, FormBreak, HpLoss, MoodHeld, NaviWinBanner, ObstacleActions, OverlayRestart, PushReading, Reactions,
            Reservations, RetypeRule, ShakeRule, StanceCounter,
        };
        assert_eq!((r.flow.result_words, r.flow.escape_check, r.flow.navi_win_banner), (49, false, NaviWinBanner::OperationBattle));
        assert_eq!(
            (r.effects.shake, r.effects.damage_word, r.effects.retype, r.effects.obstacle_actions),
            (ShakeRule::BattleRng, DamageWordRule::StatusesAndBug, RetypeRule::IsAndHits, ObstacleActions::OwnFrom6)
        );
        assert_eq!((r.chip_use.leave_on_use, r.chip_use.anti_navi_sparkle.z), (true, 16));
        assert_eq!((r.push_reading, r.overlay_restart, r.stance_counter), (PushReading::ByHitterFlip, OverlayRestart::Reload, StanceCounter::NextTick));
        assert_eq!((r.hit_test.float_shoe_needs_self_bit, r.hit_test.elec_reaches_submerged, r.hit_test.guard_breaks_to), (true, true, 0x1002));
        assert!(!r.obstacle_slide_bounds);
        assert_eq!((r.slide_speed.x, r.slide_speed.y), (0x30000, 0x20000));
        assert_eq!((r.reactions, r.form_break, r.intake.hp_loss), (Reactions::FlashTimerFirst, FormBreak::CrossOrBeast, HpLoss::HpAlone));
        let m = r.emotion;
        assert_eq!((m.mood_held, m.anger_end, m.worried_below), (MoodHeld::AtZero, AngerEnd::ResetsMood, Some(40)));
        assert_eq!((m.plain_in_battle_mode_1, m.normal_in_a_form, m.anger_before_worn_out, m.tired_and_exhausted), (false, true, false, true));
        let aura = r.effects.full_synchro_aura;
        assert_eq!((aura.follows_identity, aura.steps_while_paused, aura.stops_at_a_pause_in_the_fight), (true, true, false));
        assert_eq!((r.form_tick, r.flash_hides_on_clear, r.missing_collision_status.0, r.intake.drain_bug_flags), (false, true, 7, true));
        assert_eq!((r.pools.slots(), r.panels.reservations), ([16, 32, 8], Reservations::Unmarked));
        let f = r.fresh_stats;
        assert_eq!((f.reg_up, f.custom_level, f.mood), (7, 6, 0x70));
        assert_eq!((f.beast_out_counter, f.mode9_a), (0, None), "none stated: no Beast Out turns, no weapon");
        assert_eq!((r.panels.numbered(2), r.panels.numbered(3)), (Some(crate::field::PanelType::Normal), None), "its own numbers, no others");
        // What it may leave out reads as nothing, for every game.
        assert!(r.navicust.boards.is_empty() && r.lockon.column_shifts.is_empty() && r.chaos_cycle.is_empty() && r.holding_banners.is_empty());
        assert!(r.sp_deletion_times.is_empty() && r.sine.is_empty() && r.buster_recovery.is_empty());
        assert_eq!(r.element_weakness, [[0; 6]; 6]);
        // A section left out is a load error that names it.
        for (section, _) in STATED {
            let e = game(ruleset(Some(section), None, None)).expect_err(section);
            assert!(e.contains(&format!("game/rules/init.luau: ruleset: it states no `{section}` section")), "{section}: {e}");
            assert!(e.contains("the engine has no game's rules of its own"), "{e}");
        }
        // So is any field of one.
        let mut fields = 0;
        for (section, body) in STATED {
            for line in body.lines().filter(|l| !l.trim().is_empty()) {
                let field = line.trim_start().split(' ').next().unwrap();
                let e = game(ruleset(None, Some((section, field)), None)).expect_err(field);
                assert!(e.contains(&format!("game/rules/init.luau: ruleset: {section}: missing field `{field}`")), "{section}.{field}: {e}");
                fields += 1;
            }
        }
        assert_eq!(fields, 59, "every field of every section");
        // A field of a table of settings, too; but one that is none unless
        // stated.
        let e = game(ruleset(None, None, Some((" anger_end = \"resets_mood\",", "")))).unwrap_err();
        assert!(e.contains("ruleset: status.emotion: missing field `anger_end`"), "{e}");
        let e = game(ruleset(None, None, Some((" steps_while_paused = true,", "")))).unwrap_err();
        assert!(e.contains("ruleset: effects.full_synchro_aura: missing field `steps_while_paused`"), "{e}");
        let c = game(ruleset(None, None, Some((", worried_below = 40", "")))).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(c.rules().emotion.worried_below, None, "no mood is worried");
        // A field of a table of settings, too.
        let e = game(ruleset(None, None, Some((" elec_reaches_submerged = true,", "")))).unwrap_err();
        assert!(e.contains("ruleset: reactions.hit_test: missing field `elec_reaches_submerged`"), "{e}");
        // The fresh stats' weapon is a weapon the content defines.
        let with_weapon = |weapon: &str| -> Result<Content, String> {
            let mut c = Content::default();
            let text = ruleset(None, None, Some(("mood = 0x70,", &format!("mood = 0x70,\n        mode9_a = {weapon},"))));
            let weapon = "define.weapon { id = 'shot', charge_ticks = { 0, 0, 0, 0, 0 }, setup = function(navi) return nil :: any end }";
            let text = format!("local shot = {weapon}\n{text}");
            c.scripts.add_game("game", [("rules/init".to_string(), text)].into());
            c.define().map_err(|e| e.message)?;
            Ok(c)
        };
        let c = with_weapon("shot").unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(c.rules().fresh_stats.mode9_a, c.defs.weapon_by_key("shot"));
        assert!(c.rules().fresh_stats.mode9_a.is_some());
        let e = with_weapon("3").unwrap_err();
        assert!(e.contains("game/rules/init.luau: ruleset: fresh_stats.mode9_a: a weapon (a `define.weapon`), not"), "{e}");
        // A rule is one of the engine's, by name.
        let e = game(ruleset(None, None, Some(("shake = \"battle_rng\"", "shake = \"exe5\"")))).unwrap_err();
        assert!(e.contains("ruleset: effects.shake: unknown variant `exe5`, expected `console_rng` or `battle_rng`"), "{e}");
        let e = game(ruleset(None, None, Some(("form_break = \"cross_or_beast\"", "form_break = \"exe6\"")))).unwrap_err();
        assert!(e.contains("ruleset: status.form_break: unknown variant `exe6`, expected `cross_or_beast` or `any_form`"), "{e}");
        // A game with no ruleset states no rules: a load error too. (Modules
        // of no game, a test's, have no rules, and no battle is made of
        // them.) Rust tables state them for a content that has those (the
        // test content's), and its ruleset's sections replace them.
        let mut none = Content::default();
        none.scripts.add_game("game", [("cards".to_string(), "return define.record('card', {})".to_string())].into());
        let e = none.define().unwrap_err().message;
        assert!(e.contains("game/init.luau: game pack game defines no ruleset"), "{e}");
        let mut loose = Content::default();
        loose.scripts.add_dir("loose", [("cards".to_string(), "return define.record('card', {})".to_string())].into());
        loose.define().unwrap_or_else(|e| panic!("{}", e.message));
        assert!(loose.rules.is_none());
        let mut tables = stated();
        let pools = "return define.ruleset { pools = { actor = 4, attack = 5, effect = 6 } }";
        tables.scripts.add_game("game", [("rules/init".to_string(), pools.to_string())].into());
        tables.define().unwrap_or_else(|e| panic!("{}", e.message));
        assert_eq!(tables.rules().pools.slots(), [4, 5, 6]);
        assert_eq!(tables.rules().flow, crate::content::testing::rules().flow);
    }

    /// The user: "there should only be one ruleset per game". A game
    /// defines one, with no name and no variants: a second, an `id`, a
    /// `stock` flag and a variant's `base`, `add` and `remove` are refused,
    /// each by what it is.
    #[test]
    fn a_game_has_one_ruleset() {
        let with = |more: &[(&str, &str)]| -> Result<Content, String> {
            let mut modules = GAME.to_vec();
            modules.retain(|(p, _)| !more.iter().any(|(q, _)| q == p));
            modules.extend_from_slice(more);
            content(vec![folder("game", &modules)])
        };
        let c = with(&[]).unwrap();
        let d = &c.defs;
        let names: Vec<&str> = d.ruleset_systems().iter().map(|&h| d.system(h).key.as_str()).collect();
        assert_eq!(names, ["turns"]);
        assert_eq!(d.definitions.of(nettai_content_api::Registry::Ruleset)[0].key, nettai_content_api::RULESET_KEY);
        // Content without one: its sides have no systems.
        let none = content(vec![folder("game", &[("cards", "return define.record('card', {})")])]).unwrap();
        assert!(none.defs.ruleset().is_none() && none.defs.ruleset_systems().is_empty());
        // A second.
        let e = with(&[("rules/other", "return define.ruleset { systems = {} }")]).unwrap_err();
        assert!(e.contains("a game has one ruleset: game/rules/other.luau defines one, and game/rules/ruleset.luau another"), "{e}");
        let bad = |source: &str| -> String { with(&[("rules/ruleset", source)]).unwrap_err() };
        let cases = [
            ("return define.ruleset { id = 'stock', systems = {} }", "define.ruleset takes no `id`: a game has one ruleset"),
            ("return define.ruleset { stock = true, systems = {} }", "`stock`: a game has one ruleset"),
            ("return define.ruleset { base = {}, systems = {} }", "`base`: a game has one ruleset, and no variants of it"),
            ("return define.ruleset { add = {} }", "`add`: a game has one ruleset, and no variants of it"),
            ("return define.ruleset { remove = {} }", "`remove`: a game has one ruleset, and no variants of it"),
            ("return define.ruleset { systems = {}, game = 'game' }", "`game` is no field of a ruleset"),
            ("local t = require('./turns')\nreturn define.ruleset { systems = { t, t } }", "`systems` lists system turns twice"),
            ("return define.ruleset { systems = { 3 } }", "`systems` lists system definitions"),
        ];
        for (source, want) in cases {
            let e = bad(source);
            assert!(e.contains(want), "{source}: {e}");
        }
    }

    /// docs/design/content-model-v2.md §4.0: a load runs the game's top
    /// module, and each require finds and reads its module as it is
    /// reached; what nothing requires never runs, so it defines nothing.
    /// The order of the requires moves no key and no handle. A require of
    /// no module, a cycle, a require the packs refuse and a game without a
    /// top module are errors that say where they are written.
    #[test]
    fn a_load_reads_what_its_requires_reach() {
        let modules: BTreeMap<String, String> = [
            ("rules/turns", "return define.system { id = 'turns' }"),
            ("rules/init", "return define.ruleset { systems = { require('@self/turns') } }"),
            ("chips/cannon", "return define.record('card', { power = 3 })"),
            ("chips/sword/init", "return define.chip { id = 'sword', instant = function(u) end, parts = { define.record('part', {}), require('@self/edge') } }"),
            ("chips/sword/edge", "return define.record('part', { long = true })"),
            ("lib/pa", "return { chip = require('../chips/sword'), record = define.record('pa', {}) }"),
            ("never", "error('a module no init reaches never loads')"),
        ]
        .into_iter()
        .map(|(p, s)| (p.to_string(), s.to_string()))
        .collect();
        // The game with these inits (by path), and a support pack `lib`.
        let with_inits = |inits: &[(&str, &str)]| -> Result<Content, String> {
            let mut c = stated();
            c.scripts.add_dir("game", modules.clone());
            for (path, source) in inits {
                c.scripts.modules.insert(Scripts::name("game", path), source.to_string());
            }
            c.scripts.add_dir("lib", [("x".to_string(), "return require('@game/rules')".to_string())].into());
            c.scripts.set_manifest(PackManifest::parse("id = \"lib\"\nkind = \"support\"\n", "lib/manifest.toml")?);
            c.scripts.set_manifest(PackManifest::parse("id = \"game\"\nkind = \"game\"\ndepends = [\"lib\"]\n", "game/manifest.toml")?);
            c.define().map_err(|e| e.message)?;
            Ok(c)
        };
        let c = with_inits(&[("init", "require('@self/rules')")]).unwrap();
        assert!(c.defs.ruleset().is_some());
        assert_eq!(c.defs.record("chips/cannon#1"), None, "unreached, unloaded");
        // Each folder's init requires what the folder has; a module that
        // defines what only an id names is required the same way.
        let top = ("init", "require('@self/rules')\nrequire('@self/chips')\nrequire('@self/lib')");
        let chips = ("chips/init", "require('@self/sword')\nrequire('@self/cannon')");
        let lib = ("lib/init", "require('@self/pa')");
        let c = with_inits(&[top, chips, lib]).unwrap();
        assert!(c.defs.record("chips/cannon#1").is_some() && c.defs.chip_by_key("sword").is_some());
        // The order of the requires is the load order, and nothing more: the
        // same definitions under the same keys, so the same handles.
        let turned = [
            ("init", "require('@self/lib')\nrequire('@self/chips')\nrequire('@self/rules')"),
            ("chips/init", "require('@self/cannon')\nrequire('@self/sword')"),
            lib,
        ];
        let d = with_inits(&turned).unwrap();
        assert_eq!(c.defs.definitions, d.defs.definitions);
        // A chip its folder's init leaves out still loads when something
        // requires it (the Program Advance's module): the inits list for a
        // reader, and tools/content/index.py writes them.
        let reached = with_inits(&[top, ("chips/init", "require('@self/cannon')"), lib]).unwrap();
        assert_eq!(reached.defs.definitions, c.defs.definitions);
        let refused = |inits: &[(&str, &str)], said: &str| {
            let e = with_inits(inits).expect_err(said);
            assert!(e.contains(said), "{said}: {e}");
        };
        refused(&[("init", "require('@self/rules')\nrequire('@self/gone')")], "game/init.luau: require(\"@self/gone\"): no module game/gone.luau");
        refused(&[("init", "require('./rules')")], "game/init.luau: require(\"./rules\") from game:init: leaves pack game");
        refused(&[], "game/init.luau: game pack game has no top module");
        // A cycle, by the files it goes through.
        refused(
            &[("init", "require('@self/a')"), ("a", "return require('./b')"), ("b", "return require('./a')")],
            "require cycle: game/init.luau -> game/a.luau -> game/b.luau -> game/a.luau",
        );
        // What the packs refuse: a support pack requires no game.
        refused(&[("init", "require('@lib/x')")], "lib/x.luau: require(\"@game/rules\"): game is a game pack, which no other pack requires");
        // A game held in memory gets a top module that loads every module.
        let mut held = modules.clone();
        held.remove("never");
        let init = Scripts::init_for(&held);
        assert!(init.ends_with("require(\"@self/chips/cannon\")\nrequire(\"@self/chips/sword/edge\")\nrequire(\"@self/chips/sword\")\nrequire(\"@self/lib/pa\")\nrequire(\"@self/rules\")\nrequire(\"@self/rules/turns\")\n"), "{init}");
        let mut c = stated();
        c.scripts.add_game("game", held);
        c.define().unwrap_or_else(|e| panic!("{}", e.message));
        assert_eq!(c.defs.definitions, d.defs.definitions);
    }

    /// The define phase reads a module where the packs' folders have it, as
    /// its require is reached, unless memory holds one of its name (a
    /// tool's stand-in). What it read is the content's modules from then
    /// on: the hash covers them and not where they came from, and a runtime
    /// loads them from memory.
    #[test]
    fn a_load_reads_the_packs_folders_as_it_requires() {
        let dir = std::env::temp_dir().join(format!("nettai-scripts-{}", std::process::id()));
        let write = |path: &str, text: &str| {
            let p = dir.join(path);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, text).unwrap();
        };
        write("game/init.luau", "require(\"@self/rules\")\nrequire(\"@self/chips\")\n");
        write("game/rules/init.luau", "return define.ruleset { systems = { require(\"@self/turns\") } }\n");
        write("game/rules/turns.luau", "return define.system { id = \"turns\" }\n");
        write("game/chips/init.luau", "require(\"@self/sword\")\n");
        write("game/chips/sword/init.luau", "return define.chip { id = \"sword\", instant = require(\"@lib/hit\") }\n");
        write("game/chips/unread/init.luau", "error(\"nothing requires this\")\n");
        write("lib/hit.luau", "return function(u) end\n");
        write("lib/unread.luau", "error(\"nothing requires this\")\n");
        let on_disk = || {
            let mut c = stated();
            c.scripts.add_support_dir("lib", dir.join("lib"));
            c.scripts.set_manifest(PackManifest { id: "game".into(), kind: PackKind::Game, depends: vec!["lib".into()], extract: None });
            c.scripts.read_from("game", dir.join("game"));
            c
        };
        let mut c = on_disk();
        assert!(c.scripts.modules.is_empty(), "nothing is read ahead");
        c.define().unwrap_or_else(|e| panic!("{}", e.message));
        assert_eq!(
            c.scripts.modules.keys().collect::<Vec<_>>(),
            ["game:chips/init", "game:chips/sword/init", "game:init", "game:rules/init", "game:rules/turns", "lib:hit"],
            "what the load read"
        );
        assert!(c.defs.chip_by_key("sword").is_some());
        // The same content from memory alone is the same content.
        let mut held = stated();
        held.scripts.modules = c.scripts.modules.clone();
        held.scripts.packs = c.scripts.packs.clone();
        held.define().unwrap_or_else(|e| panic!("{}", e.message));
        assert_eq!((held.hash(), &held.defs.definitions), (c.hash(), &c.defs.definitions));
        // A runtime loads what was read, though the folder is gone.
        let mut stand_in = on_disk();
        stand_in.scripts.modules.insert("game:rules/turns".into(), "return define.system { id = \"rounds\" }\n".into());
        stand_in.define().unwrap_or_else(|e| panic!("{}", e.message));
        assert_eq!(stand_in.defs.systems[0].key, "rounds", "memory before the folder");
        // A require of no module in a file says the file.
        write("game/chips/init.luau", "require(\"@self/sword\")\nrequire(\"@self/gone\")\n");
        let e = on_disk().define().unwrap_err().message;
        assert!(e.contains("game/chips/init.luau: require(\"@self/gone\"): no module game/chips/gone.luau"), "{e}");
        std::fs::remove_dir_all(&dir).ok();
        crate::behavior::Behaviors::load(&c, Default::default()).unwrap_or_else(|e| panic!("{}", e.message));
    }

    /// A support pack defines nothing a game has.
    #[test]
    fn a_support_pack_defines_nothing_a_game_has() {
        let mut c = stated();
        c.scripts.add_support("lib", [("x".to_string(), "return define.ruleset {}".to_string())].into());
        c.scripts.add_game("game", [("chips/y".to_string(), "local _ = require('@lib/x')\nreturn define.record('y', {})".to_string())].into());
        let e = c.define().unwrap_err().message;
        assert!(e.contains("lib/x.luau: ruleset ruleset: support pack lib defines nothing a game has"), "{e}");
    }
}
