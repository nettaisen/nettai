//! The content's scripts: the Luau modules of the packs a load reads
//! (docs/design/content-model-v2.md §4.0), each named by its pack and its
//! path in it (`exe6:chips/minibomb/init`, `exelib:swords/slash`), which
//! live next to what they define.
//!
//! Each pack has a manifest (`manifest.toml`: its name, its kind and the
//! support packs it depends on), and a game pack a top module
//! (`<game>/init.luau`) that returns the game's root: what a match names,
//! by id, and its rules. The define phase runs each game's top module, and
//! each `require` finds and reads its module as it is reached
//! (`packs::find`): from the modules held in memory, else from the packs'
//! folders ([`Scripts::dirs`]); what the root reaches is what the game has. What the load
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
    /// The modules of games held in memory whose top module
    /// [`Scripts::init_for`] made (a test's), by name: a load walks each
    /// one's result as it is, besides the game's root, so what such a game
    /// holds is its whatever reaches it.
    pub held: Vec<String>,
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
            self.hold(game, modules.keys());
            modules.insert(packs::INIT.to_string(), init);
        }
        self.add_dir(game, modules);
        self.set_manifest(PackManifest { id: game.to_string(), kind: PackKind::Game, depends, text: Vec::new() });
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

    /// Game `game`'s modules `paths` (by path in its directory) are walked
    /// as they are ([`Scripts::held`]), in place of those it had.
    pub fn hold<'a>(&mut self, game: &str, paths: impl IntoIterator<Item = &'a String>) {
        let prefix = format!("{game}{}", keys::SEPARATOR);
        self.held.retain(|m| !m.starts_with(&prefix));
        self.held.extend(paths.into_iter().filter(|p| p.as_str() != packs::INIT).map(|p| Scripts::name(game, p)));
        self.held.sort();
    }

    /// A top module (`init.luau`'s text) for a game whose modules are
    /// `modules` (by path in its directory): it returns the game's root,
    /// each section ([`packs::SECTIONS`]) the tables of the modules that
    /// give it ([`packs::sections_of`]) as one, in path order, a folder by
    /// its name; each collection of the game's own a folder's init at the
    /// top gives (`patch_cards/init`: `patch_cards`); and the rules,
    /// `rules` (or `rules/init`), where there are.
    pub fn init_for(modules: &BTreeMap<String, String>) -> String {
        let mut body = String::new();
        for (section, _) in packs::SECTIONS {
            let parts: String = modules
                .iter()
                .filter(|(p, source)| p.as_str() != packs::INIT && packs::sections_of(source).contains(&section))
                .map(|(p, _)| format!("        require(\"@self/{}\"),\n", keys::listed_as(p)))
                .collect();
            if !parts.is_empty() {
                body += &format!("    {section} = merge {{\n{parts}    }},\n");
            }
        }
        for path in modules.keys() {
            let Some(dir) = path.strip_suffix("/init").filter(|d| !d.contains('/')) else { continue };
            if dir != packs::RULES && !packs::SECTIONS.iter().any(|(s, _)| *s == dir) && nettai_content_api::is_collection_name(dir) {
                body += &format!("    {dir} = require(\"@self/{dir}\"),\n");
            }
        }
        if modules.contains_key(packs::RULES) || modules.contains_key(&format!("{}/{}", packs::RULES, packs::INIT)) {
            body += &format!("    {0} = require(\"@self/{0}\"),\n", packs::RULES);
        }
        format!("{GENERATED}{MERGE}return {{\n{body}}}\n")
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
        let entries = self.entries();
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
        let entries = self.entries();
        let pack = nettai_luau::Pack::new(self.modules.clone()).with_packs(self.packs.iter().cloned()).with_compiled(self.compiled.0.clone());
        if self.packs.is_empty() { pack } else { pack.with_entries(entries) }
    }

    /// The modules a load starts from: each game's top module, then the
    /// modules [`Scripts::held`] names of it, each walked as it is.
    fn entries(&self) -> Vec<String> {
        let mut out = Vec::new();
        for p in &self.packs {
            let Some(top) = p.entry() else { continue };
            let prefix = format!("{}{}", p.id, keys::SEPARATOR);
            out.push(top);
            out.extend(self.held.iter().filter(|m| m.starts_with(&prefix)).cloned());
        }
        out
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
pub const GENERATED: &str = "-- (Made for a game held in memory: its root holds every module's definitions.)\n";

/// What an init [`Scripts::init_for`] makes merges a section's tables with
/// (content/exelib/merge.luau's, which a game held in memory may lack).
const MERGE: &str = "local function merge(parts: { { [string]: any } }): { [string]: any }
    local out = {}
    for _, part in parts do
        for id, definition in part do
            if out[id] ~= nil then
                error(string.format(\"two modules give %s\", tostring(id)), 2)
            end
            out[id] = definition
        end
    end
    return out
end

";

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
    /// `testing::rules`), for a test whose small rules name their state
    /// alone: the rules state every rule, or the content's tables do.
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
        ("rules/turns", "return { state = { n = 'u8' } }"),
        ("rules/init", "return { state = require('@self/turns').state }"),
        ("cards", "return new.record('card', { power = 1 })"),
        ("chips/cannon", "return new.record('chip-ish', { power = 3 })"),
    ];

    /// docs/design/content-model-v2.md §4.0: content holds one game pack
    /// (a match plays one game), every id written in full, its game first;
    /// one game pack requires nothing of another's.
    #[test]
    fn content_holds_one_game() {
        let mix: &[(&str, &str)] = &[
            ("rules/extra", "return { state = { e = 'u8' } }"),
            ("rules/init", "return { state = require('@self/extra').state }"),
            ("cards", "return new.record('card', { power = 2 })"),
        ];
        let c = content(vec![folder("game", GAME)]).unwrap();
        let d = &c.defs;
        assert_eq!((c.game(), d.game.as_str()), ("game", "game"));
        let rules = d.rules().expect("the game's one rules definition");
        assert!(d.schema(rules.state).index_of("n").is_some());
        // Lookups are exact, of the one game's.
        assert!(d.record("cards").is_some());
        assert_eq!(d.record("game:cards"), None);
        // Two game packs are two contents.
        let e = content(vec![folder("mix", mix), folder("game", GAME)]).unwrap_err();
        assert!(e.contains("content holds one game, and these are mix and game"), "{e}");
        // One game pack requires nothing of another's.
        let mut c = stated();
        c.scripts.add_dir("game", GAME.iter().map(|(p, s)| (p.to_string(), s.to_string())).collect());
        c.scripts.set_manifest(PackManifest { id: "game".into(), kind: PackKind::Game, ..Default::default() });
        c.scripts.add_game("mix", [("rules/init".to_string(), "return { state = require('@game/rules/turns').state }".to_string())].into());
        c.scripts.packs.retain(|p| p.id == "mix");
        let e = c.define().unwrap_err().message;
        assert!(e.contains("mix/rules/init.luau: require(\"@game/rules/turns\"): game is no pack") || e.contains("game is a game pack"), "{e}");
    }

    #[test]
    fn what_the_namespace_refuses() {
        let e = content(vec![folder("game", &[("rules/turns", "return new.collision { id = 'game:turns', side0 = 0, side1 = 0 }")])]).unwrap_err();
        assert!(e.contains("\"game:turns\" is not a valid id"), "{e}");
        // A section is the rules' field by the engine's name.
        let e = content(vec![folder("game", &[("rules/init", "return { Pools = { actor = 16 } }")])]).unwrap_err();
        assert!(e.contains("`Pools` is no field of the rules"), "{e}");
        let e = content(vec![folder("game", &[("rules/init", "return { pools = { actor = 0, attack = 32, effect = 32 } }")])]).unwrap_err();
        assert!(e.contains("game/rules/init.luau: rules: pools: a pool holds 1 to"), "{e}");
        let e = content(vec![folder("game", &[("rules/init", "return { pools = { actor = 'many', attack = 32, effect = 32 } }")])]).unwrap_err();
        assert!(e.contains("rules: pools.actor: invalid type"), "{e}");
        let e = content(vec![folder("Game", GAME)]).unwrap_err();
        assert!(e.contains("not lowercase words"), "{e}");
    }

    /// The user: "no default games anywhere please". The engine has no
    /// game's rules of its own: the rules state every rule that has no
    /// neutral value (`sections::REQUIRED`, and every field of them), and
    /// rules that leave a rule out are a load error naming it. Content with
    /// no rules definition has no rules.
    #[test]
    fn the_rules_state_every_rule() {
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
        // The sections the rules state, a field a line.
        const STATED: &[(&str, &str)] = &[
            (
                "chip_use",
                r#"
        leave_on_use = true,
        anti_navi_sparkle = { dy = 0, z = 16 },
        mixed_modifiers = false,"#,
            ),
            (
                "custom_screen",
                r#"
        slots = row(12, { kind = "hidden", keys = { "a" } }),
        emblem_at_window_return = false,
        chatbox_commands_wait_for_text = true,
        talking_characters = { only = "abc" },
        first_choosing_tick_reads_keys = false,
        run_message_at_key = true,
        invalid_picks = "refused",
        special_codes = "unstarred",
        program_advances = { once_a_round = false, keeps_regular = true, clears_past_end = true },
        modifier_passes_regular = true,
        status_until = "sending",
        hover = { runs = "while_choosing", to_dark = { {}, { music = 0x60, screen = 0xC0 } }, to_clear = { {} }, players = { 31, 9 }, sound = { counts = "while_dark", every = 61 } },
        restore_players = { 9, 31 },
        gauge_empties_at_open = false,
        fades_clear_at_ok = true,
        cursor_after_leaving = false,
        frame_counts_first = true,
        description_in_choosing = true,"#,
            ),
            (
                "effects",
                r#"
        shake = "battle_rng",
        spark_steps_at_start = true,
        retype = "is_and_hits",
        damage_word = { damage = 0x7FF, flags = { { bit = 0x4000, status = "damage_word_paralysis", hit_modifier = 0, stop = true }, { bit = 0x800, bug = 0x1118 } } },
        palette_flash = "mode_runs_through_pause",
        palette_flash_order = "after_fades",
        overlays_run_while_paused = true,
        afterimages_wear_overlays = true,
        load_sets_part_palette = true,
        obstacle_actions = "own_from_6",
        full_synchro_aura = { follows_identity = true, steps_while_paused = true, stops_at_a_pause_in_the_fight = false, spawn_runs_while_paused = false },
        charge_glow = "with_navi",
        charge = "hold_flags",
        steps = { keys = { "up", "down", "right", "left" }, confused = { up = "down", down = "up", left = "right", right = "left" }, idle_checks_target = false },
        fade_clear = "at_target",
        banner = { slide_in = 5, hold = 0x30, slide_out = 5, release = "holds_three_more", bounces = true },
        rng1_per_frame = true,
        chip_icons = "attach_point",
        used_chip_ticks = 0x3C,
        bug_flicker = true,
        panel_trail = "by_chance",
        status_visual = { place = "attach_point" },
        draw_order = "update_list","#,
            ),
            (
                "flow",
                r#"
        result_words = 49,
        sequencer_before_custom = true,
        escape_check = false,
        result_wait = { normal = 100, special = 90 },
        intro_from_black = false,
        low_hp_music = true,
        navi_win_banner = "operation_battle",
        intro_steps_on_init = false,
        sequencer_at_turn_start = true,
        custom_request = "joypads","#,
            ),
            (
                "fresh_stats",
                r#"
        reg_up = 7,
        custom_level = 6,
        mood = 0x70,"#,
            ),
            (
                "link_pick",
                r#"
        stages = {},
        first_round_stages = 0,
        backgrounds = {},
        match_stages = {},"#,
            ),
            (
                "panels",
                r#"
        types = { missing = { flags = 0 }, broken = { flags = 0 }, normal = { flags = 0x10 } },
        mend = { normal = 600, battle_mode_1 = 480 },
        start_visible = row(5, row(8, true)),
        front_edges = row(5, row(8, false)),
        numbers = { "missing", "broken", "normal" },
        roles = { missing = "missing", broken = "broken", cracked = "broken", normal = "normal" },
        step = step,
        dash_step = step,
        any_side_step = step,
        reservations = "unmarked",
        type_mask = 0x3F5F,"#,
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
        push_reading = { reads = "by_hitter_flip", bits = 4, drag_bit = 0x40, obstacle_rows = { { dx = -1, dy = 0, panels = 6 } } },
        hit_test = { float_shoe_needs_self_bit = true, bubbled_as_submerged = false, elec_reaches_submerged = true, guard_breaks_to = 0x1002, guard_before_untouchable = false, guard_marks_direction = true },
        obstacle_slide_bounds = false,
        ice = { slide = row(6, none) },
        move_direction = "by_side",
        bubble_bob = row(32, 0),
        slide_speed = { x = 0x30000, y = 0x20000 },
        overlay_restart = "reload",
        stance_counter = "next_tick",
        dead_player = "kept",
        attack_end_lockouts = true,
        request_clears = { attack = { "mode9_a" }, paralysis = { "mode9_a" }, flinch = { "anti_sword_triggered" }, drag = { 0x400 } },"#,
            ),
            (
                "status",
                r#"
        hp_drain = { periods = row(8, 30), stops_while_paused = true },
        custom_drain = { periods = "stat", status = 5 },
        form_tick = false,
        flash_hides_on_clear = true,
        missing_collision_status = 7,
        reactions = "flash_timer_first",
        reaction_actions = "marked",
        drag = { poses = { otherwise = 1 }, ending = "stands" },
        bugs_before_drain = false,
        no_charge_drive = false,
        hp_loss = "hp_alone",
        hit_sound = "by_console",
        barrier = { stops_while_paused = true, wind = "pops" },
        emotion = { mood_held = "at_zero", anger_end = "resets_mood", order = { { emotion = "worn_out", when = { { mood = 0 }, { exhausted = true } } }, { emotion = "worried", when = { { mood_below = 40, in_form = false } } }, { emotion = "normal" } }, roles = { worn_out = "worn_out" }, hit_mood = "hitter_gains", full_synchro_spent = 0x99, anger_boost_sound = false },
        form_break = "marked_forms",
        weakness_hit_breaks_form = true,
        weakness_mark = "weak_element_damage",
        paused_navi = "pause_handler",
        timers_while_paused = false,
        idle_stands = false,"#,
            ),
        ];
        assert_eq!(STATED.iter().map(|(name, _)| *name).collect::<Vec<_>>(), crate::content::sections::REQUIRED);
        // The rules, without a section or a field of one, or with a
        // field's value another.
        let rules = |without: Option<&str>, field: Option<(&str, &str)>, other: Option<(&str, &str)>| -> String {
            let mut out = format!("{HEAD}return {{\n");
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
        let game = |rules: String| -> Result<Content, String> {
            let mut c = Content::default();
            c.scripts.add_game("game", [("rules/init".to_string(), rules)].into());
            c.define().map_err(|e| e.message)?;
            Ok(c)
        };
        // Every rule stated: the game's rules are what it states, each a
        // choice of its own (no game has these together).
        let c = game(rules(None, None, None)).unwrap_or_else(|e| panic!("{e}"));
        let r = c.rules();
        use crate::content::{
            AngerEnd, FormBreak, HpLoss, MoodHeld, NaviWinBanner, ObstacleActions, OverlayRestart, PushSource, Reactions,
            Reservations, RetypeRule, ShakeRule, StanceCounter, WeaknessMark,
        };
        assert_eq!((r.flow.result_words, r.flow.escape_check, r.flow.navi_win_banner), (49, false, NaviWinBanner::OperationBattle));
        assert_eq!(r.flow.custom_request, crate::content::CustomRequest::Joypads);
        assert_eq!(
            (r.effects.shake, r.effects.damage_word.damage, r.effects.damage_word.flags.len(), r.effects.retype, r.effects.obstacle_actions),
            (ShakeRule::BattleRng, 0x7FF, 2, RetypeRule::IsAndHits, ObstacleActions::OwnFrom6)
        );
        assert_eq!((r.chip_use.leave_on_use, r.chip_use.anti_navi_sparkle.z), (true, 16));
        let s = &r.custom_screen;
        assert_eq!((s.first_choosing_tick_reads_keys, s.status_until), (false, crate::content::StatusUntil::Sending));
        assert!(matches!(&s.slots[11].moves, crate::content::SlotMoves::Candidates { keys, up, .. } if keys.len() == 1 && up.is_empty()));
        assert_eq!((s.hover.to_dark.clone(), s.hover.to_clear.len(), s.hover.players, s.restore_players.clone()), (vec![None, Some([0x60, 0xC0])], 1, [31, 9], vec![9, 31]));
        assert_eq!((s.hover.runs, s.hover.sound), (crate::content::HoverRuns::WhileChoosing, crate::content::HoverSound::WhileDark { every: 61 }));
        assert_eq!((r.push_reading.reads, r.push_reading.bits, r.overlay_restart, r.stance_counter), (PushSource::ByHitterFlip, 4, OverlayRestart::Reload, StanceCounter::NextTick));
        assert_eq!((r.hit_test.float_shoe_needs_self_bit, r.hit_test.elec_reaches_submerged, r.hit_test.guard_breaks_to), (true, true, 0x1002));
        assert!(!r.obstacle_slide_bounds);
        assert_eq!((r.slide_speed.x, r.slide_speed.y), (0x30000, 0x20000));
        assert_eq!((r.reactions, r.form_break, r.intake.hp_loss), (Reactions::FlashTimerFirst, FormBreak::MarkedForms, HpLoss::HpAlone));
        assert_eq!((r.weakness_hit_breaks_form, r.weakness_mark), (true, WeaknessMark::WeakElementDamage));
        let m = &r.emotion;
        assert_eq!((m.mood_held, m.anger_end, m.names.clone()), (MoodHeld::AtZero, AngerEnd::ResetsMood, vec!["normal".to_string(), "worn_out".into(), "worried".into()]));
        assert_eq!((m.hit_mood, m.full_synchro_spent, m.anger_boost_sound), (crate::content::HitMood::HitterGains, 0x99, false));
        let worried = crate::content::EmotionWhen { mood_below: Some(40), in_form: Some(false), ..Default::default() };
        assert_eq!((m.order.len(), m.order[1].when.clone(), m.role(crate::content::Emotion(1)), m.role(crate::content::Emotion(0))), (3, vec![worried], Some(crate::content::EmotionRole::WornOut), None));
        let aura = r.effects.full_synchro_aura;
        assert_eq!((aura.follows_identity, aura.steps_while_paused, aura.stops_at_a_pause_in_the_fight, aura.spawn_runs_while_paused), (true, true, false, false));
        assert_eq!((r.form_tick, r.flash_hides_on_clear, r.missing_collision_status.0), (false, true, 7));
        assert_eq!((r.pools.slots(), r.panels.reservations), ([16, 32, 8], Reservations::Unmarked));
        let f = r.fresh_stats;
        assert_eq!((f.reg_up, f.custom_level, f.mood), (7, 6, 0x70));
        assert_eq!((f.stats, f.mode9_a), (Default::default(), None), "none stated: none of the game's own, no weapon");
        assert_eq!((r.panels.numbered(2), r.panels.numbered(3)), (Some(crate::field::PanelType(2)), None), "its own numbers, no others");
        // What it may leave out reads as nothing, for every game.
        assert!(r.holding_banners.is_empty());
        assert!(r.sine.is_empty() && r.buster_recovery.is_empty());
        assert_eq!(r.element_weakness, [[0; 6]; 6]);
        // A section left out is a load error that names it.
        for (section, _) in STATED {
            let e = game(rules(Some(section), None, None)).expect_err(section);
            assert!(e.contains(&format!("game/rules/init.luau: rules: it states no `{section}` section")), "{section}: {e}");
            assert!(e.contains("the engine has no game's rules of its own"), "{e}");
        }
        // So is any field of one.
        let mut fields = 0;
        for (section, body) in STATED {
            for line in body.lines().filter(|l| !l.trim().is_empty()) {
                let field = line.trim_start().split(' ').next().unwrap();
                let e = game(rules(None, Some((section, field)), None)).expect_err(field);
                assert!(e.contains(&format!("game/rules/init.luau: rules: {section}: missing field `{field}`")), "{section}.{field}: {e}");
                fields += 1;
            }
        }
        assert_eq!(fields, 108, "every field of every section");
        // A field of a table of settings, too; but one that is none unless
        // stated.
        let e = game(rules(None, None, Some((" anger_end = \"resets_mood\",", "")))).unwrap_err();
        assert!(e.contains("rules: status.emotion: missing field `anger_end`"), "{e}");
        let e = game(rules(None, None, Some((" steps_while_paused = true,", "")))).unwrap_err();
        assert!(e.contains("rules: effects.full_synchro_aura: missing field `steps_while_paused`"), "{e}");
        // The order's last case holds always; a role is of an emotion the order names.
        let e = game(rules(None, None, Some(("{ emotion = \"normal\" }", "{ emotion = \"normal\", when = { { mood = 1 } } }")))).unwrap_err();
        assert!(e.contains("the emotions' order ends with `normal` when it holds: its last case holds always"), "{e}");
        let e = game(rules(None, None, Some(("roles = { worn_out", "roles = { tired")))).unwrap_err();
        assert!(e.contains("roles: `tired` is no emotion of the order's"), "{e}");
        // A field of a table of settings, too.
        let e = game(rules(None, None, Some((" elec_reaches_submerged = true,", "")))).unwrap_err();
        assert!(e.contains("rules: reactions.hit_test: missing field `elec_reaches_submerged`"), "{e}");
        // The fresh stats' weapon is a weapon the content defines.
        let with_weapon = |weapon: &str| -> Result<Content, String> {
            let mut c = Content::default();
            let text = rules(None, None, Some(("mood = 0x70,", &format!("mood = 0x70,\n        mode9_a = {weapon},"))));
            let weapon = "new.weapon { id = 'shot', charge_ticks = { 0, 0, 0, 0, 0 }, setup = function(navi) return nil :: any end }";
            let text = format!("local shot = {weapon}\n{text}");
            c.scripts.add_game("game", [("rules/init".to_string(), text)].into());
            c.define().map_err(|e| e.message)?;
            Ok(c)
        };
        let c = with_weapon("shot").unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(c.rules().fresh_stats.mode9_a, c.defs.weapon_by_key("shot"));
        assert!(c.rules().fresh_stats.mode9_a.is_some());
        let e = with_weapon("3").unwrap_err();
        assert!(e.contains("game/rules/init.luau: rules: fresh_stats.mode9_a: a weapon (a `new.weapon`), not"), "{e}");
        // A rule is one of the engine's, by name.
        let e = game(rules(None, None, Some(("shake = \"battle_rng\"", "shake = \"exe5\"")))).unwrap_err();
        assert!(e.contains("rules: effects.shake: unknown variant `exe5`, expected `console_rng` or `battle_rng`"), "{e}");
        let e = game(rules(None, None, Some(("form_break = \"marked_forms\"", "form_break = \"exe6\"")))).unwrap_err();
        assert!(e.contains("rules: status.form_break: unknown variant `exe6`, expected `marked_forms` or `any_form`"), "{e}");
        // A game with no rules states no rules: a load error too. (Modules
        // of no game, a test's, have no rules, and no battle is made of
        // them.) Rust tables state them for a content that has those (the
        // test content's), and its rules' sections replace them.
        let mut none = Content::default();
        none.scripts.add_game("game", [("cards".to_string(), "return new.record('card', {})".to_string())].into());
        let e = none.define().unwrap_err().message;
        assert!(e.contains("game/init.luau: game pack game defines no rules"), "{e}");
        let mut loose = Content::default();
        loose.scripts.add_dir("loose", [("cards".to_string(), "return new.record('card', {})".to_string())].into());
        loose.define().unwrap_or_else(|e| panic!("{}", e.message));
        assert!(loose.rules.is_none());
        let mut tables = stated();
        let pools = "return { pools = { actor = 4, attack = 5, effect = 6 } }";
        tables.scripts.add_game("game", [("rules/init".to_string(), pools.to_string())].into());
        tables.define().unwrap_or_else(|e| panic!("{}", e.message));
        assert_eq!(tables.rules().pools.slots(), [4, 5, 6]);
        assert_eq!(tables.rules().flow, crate::content::testing::rules().flow);
    }

    /// The user: "there should only be one rules definition per game", then "collapse
    /// systems into one rules definition". A game's root holds one, its
    /// `rules`, with no name and no variants: a second root's, an `id`, and
    /// a field that is none of the rules' (a `stock` flag, a variant's
    /// `base`, `add` and `remove`) are refused.
    #[test]
    fn a_game_has_one_rules_definition() {
        let with = |more: &[(&str, &str)]| -> Result<Content, String> {
            let mut modules = GAME.to_vec();
            modules.retain(|(p, _)| !more.iter().any(|(q, _)| q == p));
            modules.extend_from_slice(more);
            content(vec![folder("game", &modules)])
        };
        let c = with(&[]).unwrap();
        let d = &c.defs;
        assert!(d.rules().is_some());
        assert_eq!(d.definitions.of(nettai_content_api::Registry::Rules)[0].key, nettai_content_api::RULESET_KEY);
        // Content without one: its sides have no rules.
        let none = content(vec![folder("game", &[("cards", "return new.record('card', {})")])]).unwrap();
        assert!(none.defs.rules().is_none());
        // A second root's.
        let e = with(&[("rules/other", "return { rules = {} }")]).unwrap_err();
        assert!(e.contains("a game has one rules definition"), "{e}");
        let bad = |source: &str| -> String { with(&[("rules/init", source)]).unwrap_err() };
        let cases = [
            ("return { id = 'stock' }", "`id` is no field of the rules"),
            ("return { stock = true }", "`stock` is no field of the rules"),
            ("return { base = {} }", "`base` is no field of the rules"),
            ("return { add = {} }", "`add` is no field of the rules"),
            ("return { remove = {} }", "`remove` is no field of the rules"),
            ("return { game = 'game' }", "`game` is no field of the rules"),
            ("return { systems = {} }", "`systems` is no field of the rules"),
        ];
        for (source, want) in cases {
            let e = bad(source);
            assert!(e.contains(want), "{source}: {e}");
        }
    }

    /// docs/design/content-model-v2.md §4.0: a load runs the game's top
    /// module, and each require finds and reads its module as it is
    /// reached; what nothing requires never runs. What the game has is what
    /// the root the top module returns reaches: a module that runs makes
    /// nothing by running (no side effect). The order of the requires and
    /// of the root's fields moves no key and no handle. A require of no
    /// module, a cycle, a require the packs refuse and a game without a top
    /// module are errors that say where they are written.
    #[test]
    fn a_load_reads_what_its_requires_reach() {
        let modules: BTreeMap<String, String> = [
            ("rules/turns", "return { state = { n = 'u8' } }"),
            ("rules/init", "return { state = require('@self/turns').state }"),
            ("chips/cannon", "local cannon: Chip = { instant = function(u) end, part = new.record('part', {}) }\nreturn { cannon = cannon }"),
            (
                "chips/sword/init",
                "local sword: Chip = { instant = function(u) end, parts = { new.record('part', {}), require('@self/edge') } }\nreturn { sword = sword }",
            ),
            ("chips/sword/edge", "return new.record('part', { long = true })"),
            ("lib/pa", "return { chip = require('../chips/sword').sword, record = new.record('pa', {}) }"),
            ("never", "error('a module nothing requires never loads')"),
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
        let c = with_inits(&[("init", "return { rules = require('@self/rules') }")]).unwrap();
        assert!(c.defs.rules().is_some());
        assert_eq!(c.defs.chip_by_key("cannon"), None, "unreached, unloaded");
        // A section's init returns the section; what the root reaches is
        // the game's, and a module that only runs makes nothing.
        let top = ("init", "local _ = require('@self/lib/pa')\nreturn { rules = require('@self/rules'), chips = require('@self/chips') }");
        let chips = ("chips/init", "local sword = require('@self/sword')\nlocal cannon = require('@self/cannon')\nreturn { sword = sword.sword, cannon = cannon.cannon }");
        let c = with_inits(&[top, chips]).unwrap();
        assert!(c.defs.chip_by_key("cannon").is_some() && c.defs.chip_by_key("sword").is_some());
        assert!(c.defs.record("cannon/part").is_some() && c.defs.record("sword/parts/1").is_some());
        // (A shared table is named by the module that writes it.)
        assert!(c.defs.record("chips/sword/edge").is_some());
        assert_eq!(c.defs.record("lib/pa/record"), None, "loaded, and reached by nothing");
        // The order of the requires is the load order, and nothing more: the
        // same definitions under the same keys, so the same handles.
        let turned = [
            ("init", "return { chips = require('@self/chips'), rules = require('@self/rules') }"),
            ("chips/init", "local cannon = require('@self/cannon')\nlocal sword = require('@self/sword')\nreturn { cannon = cannon.cannon, sword = sword.sword }"),
        ];
        let d = with_inits(&turned).unwrap();
        assert_eq!(c.defs.definitions, d.defs.definitions);
        let refused = |inits: &[(&str, &str)], said: &str| {
            let e = with_inits(inits).expect_err(said);
            assert!(e.contains(said), "{said}: {e}");
        };
        refused(&[("init", "local _ = require('@self/gone')\nreturn { rules = require('@self/rules') }")], "game/init.luau: require(\"@self/gone\"): no module game/gone.luau");
        refused(&[("init", "return { rules = require('./rules') }")], "game/init.luau: require(\"./rules\") from game:init: leaves pack game");
        refused(&[], "game/init.luau: game pack game has no top module");
        // The root's other keys are the game's own collections, each a
        // table of entries by id (data, keyed `<collection>/<id>`).
        let held_cards = with_inits(&[("init", "return { rules = require('@self/rules'), cards = { joker = { power = 1 } } }")]).unwrap();
        let joker = held_cards.defs.entry_in("cards", "joker").expect("an entry of the collection");
        assert_eq!((held_cards.defs.entry(joker).key.as_str(), held_cards.defs.collections()), ("cards/joker", vec!["cards"]));
        refused(&[("init", "return { rules = require('@self/rules'), cards = 3 }")], "game/init.luau: `cards` is a table of entries by id, not integer");
        refused(&[("init", "return { rules = require('@self/rules'), Cards = {} }")], "game/init.luau: `Cards` names no collection");
        // A cycle, by the files it goes through.
        refused(
            &[("init", "return require('@self/a')"), ("a", "return require('./b')"), ("b", "return require('./a')")],
            "require cycle: game/init.luau -> game/a.luau -> game/b.luau -> game/a.luau",
        );
        // What the packs refuse: a support pack requires no game.
        refused(&[("init", "local _ = require('@lib/x')\nreturn {}")], "lib/x.luau: require(\"@game/rules\"): game is a game pack, which no other pack requires");
        // A game held in memory gets a top module whose root holds every
        // module's sections, and the load walks its other modules too.
        let mut held = modules.clone();
        held.remove("never");
        let init = Scripts::init_for(&held);
        assert!(
            init.contains("    chips = merge {\n        require(\"@self/chips/cannon\"),\n        require(\"@self/chips/sword\"),\n    },\n")
                && init.contains("    rules = require(\"@self/rules\"),\n"),
            "{init}"
        );
        let mut h = stated();
        h.scripts.add_game("game", held);
        h.define().unwrap_or_else(|e| panic!("{}", e.message));
        assert_eq!(h.defs.chips, c.defs.chips);
        assert!(h.defs.record("lib/pa/record").is_some(), "a held module, walked");
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
        write("game/init.luau", "return { rules = require(\"@self/rules\"), chips = require(\"@self/chips\") }\n");
        write("game/rules/init.luau", "return { state = require(\"@self/turns\").state }\n");
        write("game/rules/turns.luau", "return { state = { n = \"u8\" } }\n");
        write("game/chips/init.luau", "return { sword = require(\"@self/sword\").sword }\n");
        write("game/chips/sword/init.luau", "return { sword = { instant = require(\"@lib/hit\") } }\n");
        write("game/chips/unread/init.luau", "error(\"nothing requires this\")\n");
        write("lib/hit.luau", "return function(u) end\n");
        write("lib/unread.luau", "error(\"nothing requires this\")\n");
        let on_disk = || {
            let mut c = stated();
            c.scripts.add_support_dir("lib", dir.join("lib"));
            c.scripts.set_manifest(PackManifest { id: "game".into(), kind: PackKind::Game, depends: vec!["lib".into()], text: Vec::new() });
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
        stand_in.scripts.modules.insert("game:rules/turns".into(), "return { state = { r = 'u8' } }\n".into());
        stand_in.define().unwrap_or_else(|e| panic!("{}", e.message));
        let state = stand_in.defs.schema(stand_in.defs.rules().expect("the rules").state);
        assert!(state.index_of("r").is_some(), "memory before the folder");
        // A require of no module in a file says the file.
        write("game/chips/init.luau", "local _ = require(\"@self/gone\")\nreturn { sword = require(\"@self/sword\").sword }\n");
        let e = on_disk().define().unwrap_err().message;
        assert!(e.contains("game/chips/init.luau: require(\"@self/gone\"): no module game/chips/gone.luau"), "{e}");
        std::fs::remove_dir_all(&dir).ok();
        crate::behavior::Behaviors::load(&c, Default::default()).unwrap_or_else(|e| panic!("{}", e.message));
    }

    /// A support pack defines nothing a game has: no table of the root's is
    /// written in one.
    #[test]
    fn a_support_pack_defines_nothing_a_game_has() {
        let mut c = stated();
        c.scripts.add_support("lib", [("x".to_string(), "return { power = 1 }".to_string())].into());
        c.scripts.add_game("game", [("init".to_string(), "return { chips = { y = require('@lib/x') } }".to_string())].into());
        let e = c.define().unwrap_err().message;
        assert!(e.contains("lib/x.luau: chip y: support pack lib defines nothing a game has"), "{e}");
    }
}
