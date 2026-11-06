//! The content (content/) type-checks, each pack against its
//! declarations; its requires name only modules that are there, each
//! require one its pack may make; and misuse of the API is a type error.

use std::path::Path;

fn pack() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")
}

#[test]
fn the_content_type_checks_against_the_core_api() {
    let (checked, problems) = nettai_content_check::check_content(&pack()).unwrap();
    assert!(checked >= 1000, "found the content's modules ({checked})");
    assert!(problems.is_empty(), "type errors:\n{}", problems.join("\n"));
}

/// docs/design/content-model-v2.md §4.0: a game without its top module, a
/// require of no module, a folder that is no pack, a require of another
/// game's module (game to game, support to game), a support pack reaching
/// for the game's context by itself and a cycle of support packs fail the
/// check with their paths.
#[test]
fn what_the_packs_refuse() {
    let dir = std::env::temp_dir().join(format!("nettai-check-{}", std::process::id()));
    let write = |path: &str, text: &str| {
        let p = dir.join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    };
    write("g/manifest.toml", "id = \"g\"\nkind = \"game\"\ndepends = [\"lib\"]\n");
    write("g/init.luau", "require(\"@self/there\")\nrequire(\"@self/gone\")\nrequire(\"./there\")\nrequire(\"@self/chips\")\n");
    // (A folder's name is its init; a commented require is none.)
    write("g/chips/init.luau", "require(\"@self/sword\")\n-- require(\"@self/later\")  -- no use yet\n");
    write("g/chips/sword/init.luau", "return { sword = {} }\n");
    write("g/there.luau", "local y = require(\"./nowhere\")\nlocal z = require(\"@h/x\")\nlocal w = require(\"@lib/x\")\nreturn {}\n");
    write("h/manifest.toml", "id = \"h\"\nkind = \"game\"\n");
    write("h/x.luau", "return {}\n");
    write("lib/manifest.toml", "id = \"lib\"\nkind = \"support\"\ndepends = [\"base\"]\n");
    write(
        "lib/x.luau",
        "local g = require(\"@g/there\")\nlocal _ = asset.sprite(\"x\")\nlocal s = rules.state\nreturn new.kind { id = \"x\" }\n",
    );
    write("base/manifest.toml", "id = \"base\"\nkind = \"support\"\ndepends = [\"lib\"]\n");
    write("stray/x.luau", "return {}\n");
    let problems = nettai_content_check::reach(&dir).unwrap();
    let has = |want: &str| assert!(problems.iter().any(|p| p.contains(want)), "{want}: {problems:#?}");
    has("g/init.luau: require(\"@self/gone\"): no module g/gone.luau");
    has("g/init.luau: require(\"./there\") from g:init: leaves pack g");
    has("h/init.luau: game pack h has no top module: its init.luau requires what the game has");
    has("g/there.luau: require(\"./nowhere\"): no module g/nowhere.luau");
    has("g/there.luau: require(\"@h/x\"): h is a game pack, which no other pack requires");
    has("lib/x.luau: require(\"@g/there\"): g is a game pack, which no other pack requires");
    has("lib/x.luau:2: support pack lib reaches for the game's context by itself, `asset`");
    has("lib/x.luau:3: support pack lib reaches for the game's context by itself, `rules`");
    has("lib/x.luau:4: support pack lib reaches for the game's context by itself, `id = \"x\"`");
    has("its `depends` make a cycle: base depends on lib depends on base");
    has("stray/: no manifest.toml; a folder of content/ is a pack");
    assert!(!problems.iter().any(|p| p.starts_with("g/chips/")), "the folder's init and its requires are found: {problems:#?}");
    std::fs::remove_dir_all(&dir).ok();
}

/// The engine's test pack and test content (crates/nettai-battle/testdata)
/// type-check against the same definitions, and pass the lints.
#[test]
fn the_test_pack_and_test_content_type_check() {
    let mut checker = nettai_content_check::PackChecker::new(&nettai_content_check::definitions(&pack(), "exe6").unwrap()).unwrap();
    let mut problems = Vec::new();
    for (name, at_least) in [("pack", 5), ("content", 5)] {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../nettai-battle/testdata").join(name);
        let modules = nettai_content_check::modules(&dir).unwrap();
        assert!(modules.len() >= at_least, "testdata/{name}: {} modules", modules.len());
        for (path, source) in &modules {
            let full = format!("testdata/{name}/{path}");
            problems.extend(checker.check(&full, source).unwrap());
            // (The lints read a module's path in its root: rules/ holds a
            // game's rules.)
            problems.extend(nettai_content_check::lints::lints(path, source).into_iter().map(|p| format!("testdata/{name}/{p}")));
        }
    }
    assert!(problems.is_empty(), "problems:\n{}", problems.join("\n"));
}

/// Small rules for a test: the sections every game states (each any
/// table, to the type), then `more` (its fields, each ending in a comma).
fn rules(more: &str) -> String {
    format!(
        "local _: Rules = {{ chip_use = {{}}, effects = {{}}, flow = {{}}, fresh_stats = {{}}, link_pick = {{}}, panels = {{}}, pools = {{}}, reactions = {{}}, status = {{}}, {more} }}"
    )
}

/// docs/design/content-model-v2.md §3.8: the rules state `chip_use`,
/// `effects`, `flow`, `fresh_stats`, `link_pick`, `panels`, `pools`, `reactions` and
/// `status`, and the type says so: rules without any of them don't type-check, whatever else
/// it states (the load says which field of one is missing).
#[test]
fn rules_without_a_required_section_are_a_type_error() {
    let mut checker = nettai_content_check::PackChecker::new(&nettai_content_check::definitions(&pack(), "exe6").unwrap()).unwrap();
    let mut check = |source: &str| checker.check("rules/init.luau", &format!("--!strict\n{source}\n")).unwrap();
    let whole = rules("math = { sine = {} },");
    assert_eq!(check(&whole), Vec::<String>::new(), "every required section stated");
    for section in ["chip_use", "effects", "flow", "fresh_stats", "link_pick", "panels", "pools", "reactions", "status"] {
        let without = whole.replace(&format!("{section} = {{}}, "), "");
        assert_ne!(without, whole);
        let problems = check(&without);
        assert!(problems.iter().any(|p| p.contains(section)), "without `{section}`: {problems:?}");
    }
    // A section a game may leave out is no problem, nor are no parts.
    assert_eq!(check(&rules("")), Vec::<String>::new());
}

#[test]
fn misuse_of_the_v2_api_is_a_type_error() {
    let mut checker = nettai_content_check::PackChecker::new(&nettai_content_check::definitions(&pack(), "exe6").unwrap()).unwrap();
    for (bad, why) in [
        ("local _ = new.kind { id = 'x', pool = 'water', update = function(me: Object) end }", "not a pool"),
        ("local _ = new.kind { id = 'x', pool = 'attack' }", "a kind without its update"),
        ("local _ = asset.sprite(3)", "an asset by number"),
        ("local _ = new.effect { sprite = 'bomb' }", "a sprite by string"),
        ("local function f(me: Object) me:set_attack('shot', \"buster\") end", "an action by name"),
        ("local function f(me: Object) local _ = battle.spawn(me, me.pos) end", "an object for a kind"),
        ("local _ = new.region { panels = { 'front' } }", "a panel that isn't { dx, dy }"),
    ] {
        let problems = checker.check(why, &format!("--!strict\n{bad}\n")).unwrap();
        assert!(!problems.is_empty(), "{why}: `{bad}` should not type-check");
    }
    // (Rules with its sections, so the role is what is wrong.)
    let bad = rules("roles = { actions = { anti_damage_counter = 3 } },");
    let problems = checker.check("a role that isn't an action", &format!("--!strict\n{bad}\n")).unwrap();
    assert!(problems.len() == 1 && problems[0].contains("Action"), "`{bad}`: {problems:?}");
}

#[test]
fn the_numeric_api_that_is_gone_is_a_type_error() {
    let mut checker = nettai_content_check::PackChecker::new(&nettai_content_check::definitions(&pack(), "exe6").unwrap()).unwrap();
    for (bad, why) in [
        ("local _ = battle.spawn(\"attack\", 8)", "a spawn by pool and index"),
        ("local _ = battle.spawn_kind(\"rock\")", "a spawn by name"),
        ("local function f(me: Object) local _ = me:param(1) end", "a spawn parameter"),
        ("local function f(me: Object) local _ = me:attack_param(1) end", "an attack parameter"),
        ("local function f(me: Object) local _ = me.index end", "a kind by index"),
        ("local function f(me: Object) local _ = me.variant end", "the attack's variant"),
        ("local function f(me: Object) local _: number = me.chip end", "the attack's chip by number"),
        ("local function f(me: Object) me:set_attack(0x12, \"chip\") end", "an action by number"),
        ("local function f(me: Object) me.sprite:load(\"0c-01\") end", "a sprite by number"),
        // (`battle.hand_chip(side, i)` is a hand's chip by handle, a `Chip`: the number it gave is what is gone.)
        ("local _: number = battle.hand_chip(0, 0)", "a hand's chip by number"),
        ("local function f(me: Object) dimming.show_navi_telop(me, 0x123) end", "a telop's chip by number"),
        ("battle.set_linked(0, { chip = 0x123, bonus = 0, damage = 0 })", "a linked record's chip by number"),
        ("local _ = data.rules.sine[1]", "a rule table by number"),
        ("local function f(me: Object) local _ = battle.effect(me.pos, 3) end", "an effect by number"),
        ("battle.play_sound(0x10)", "a sound by number"),
        ("local function f(me: Object) local _ = me.name_id end", "an object's NameID"),
        ("local _ = battle.navi_record(0x1A0)", "an actor record by NameID"),
        ("local function f(me: Object) me:death_hook(0x1A0) end", "a death hook by NameID"),
        ("local _ = battle.attach_point(0x1A0, 3, 0, 0)", "an attach point by NameID"),
        ("local function f(me: Object) me:setup_collision(4, 5, 0) end", "collision types by number"),
        ("local _ = new.collision { side0 = 0x80000080, side1 = 0x40000080, row_offset = 0x20 }", "a collision type by words"),
        ("local _ = field.flags(1, 1)", "a panel's flags word"),
        ("local _ = field.check(1, 1, 0x10, 0x0F880080)", "a panel condition by words"),
        ("local _ = field.check(1, 1, \"solid\", \"occupied\")", "a panel condition by names, required and forbidden"),
        ("local _ = field.any(1, 1, \"neutral\")", "a set of panel flags by a name"),
        ("local _ = new.region { field = { require = { \"solid\" } } }", "a region's condition by lists of names"),
        ("local _ = new.collision { flags = { \"own_attack\" } }", "a collision type's members as a list"),
        ("local function f(me: Object) local _ = me.collision:hit_any(\"guarded\") end", "a hit's name by itself"),
        ("local function f(me: Object) local _ = me.collision:hit_by() end", "the hitters by `hit_by`"),
        ("local _: StageDef = { layout = {}, panel_pattern = 0x38, mode = 0, battle_number = 0, effects = { \"link\" }, actors = {} }", "a stage's effects as a list"),
        ("local function f(c: ChipSpec) c.flags = { \"has_damage\" } end", "a chip's flags as a list"),
        ("local _: boolean = battle.has_effect(\"karma_held\")", "a battle effect by a call"),
        ("local _ = new.region { field = { require = 0x20, forbid = 0 } }", "a region's condition by words"),
        ("local function f(me: Object) local _ = me.collision.hit_flags end", "hit results as a word"),
        // A status raises its timer's request; the rows past the original's table, a collision offset, are gone.
        ("local _: StatusDef = { duration = 90, timer = { other = 10 } }", "a status timer by collision offset"),
        ("local _ = battle.effects()", "the battle's effects as a word"),
        ("local _: StageDef = { layout = {}, panel_pattern = 0x38, mode = 0, battle_number = 0, effects = 0x88C, actors = {} }", "a stage's effects as a word"),
        ("local function f(c: ChipSpec) c.extra_flags = { 128 } end", "a chip's extra flag by number"),
    ] {
        let problems = checker.check(why, &format!("--!strict\n{bad}\n")).unwrap();
        assert!(!problems.is_empty(), "{why}: `{bad}` should not type-check");
    }
    // (Rules with their sections, so the role is what is wrong.)
    for (roles, why) in [
        ("roles = { actions = { turn = { legacy = { action = 0x3B } } } },", "a role by action number"),
        ("roles = { kinds = { support = { legacy = { kind = \"support\" } } } },", "a role by kind key"),
    ] {
        let bad = rules(roles);
        let problems = checker.check(why, &format!("--!strict\n{bad}\n")).unwrap();
        assert!(problems.len() == 1 && !problems[0].contains("chip_use"), "{why}: `{bad}`: {problems:?}");
    }
}

/// docs/design/scripting.md §2.4: a state's field types are `schema` values; a type's name, a string, is no
/// type, nor is a record type's.
#[test]
fn a_type_by_name_is_a_type_error() {
    let mut checker = nettai_content_check::PackChecker::new(&nettai_content_check::definitions(&pack(), "exe6").unwrap()).unwrap();
    let kind = |state: &str| format!("local _ = new.kind {{ pool = \"attack\", update = function(me: Object) end, state = {state} }}");
    let good = [
        kind("{ n = schema.u8, a = schema.array(schema.u8, 18), c = schema.chip, o = schema.optional(schema.u8), v = { \"a\", \"b\" } }"),
        kind("{ l = schema.list({ chip = schema.chip, code = schema.code }, 30), r = schema.role(\"form_list\", schema.array(schema.form, 5)) }"),
        "local T = schema.record_type(\"t\")\nlocal _ = new.record(T, { n = 1 })\nlocal _ = new.kind { pool = \"attack\", update = function(me: Object) end, state = { t = T, e = schema.entry(\"patch_cards\") } }".to_string(),
        "local _ = new.kind { pool = \"attack\", update = function(me: Object) end, state = { look = obstacle.absorbed_look } }".to_string(),
    ];
    for source in good {
        let problems = checker.check("good", &format!("--!strict\n{source}\n")).unwrap();
        assert!(problems.is_empty(), "`{source}`: {problems:?}");
    }
    for (bad, why) in [
        (kind("{ n = \"u8\" }"), "a scalar by name"),
        (kind("{ a = \"u8[18]\" }"), "an array by name"),
        (kind("{ c = \"chip\" }"), "a reference by name"),
        (kind("{ r = \"record:bomb-variant\" }"), "a record type by name"),
        (kind("{ l = schema.list(\"u8\", 2) }"), "a list's elements by name"),
        (kind("{ r = schema.role(\"form_list\", \"u8\") }"), "a role's type by name"),
        ("local _ = schema.array(\"u8\", 3)".to_string(), "an array's element by name"),
        ("local _ = new.record(\"bomb-variant\", {})".to_string(), "a record of a type by name"),
        ("local _ = new.record(schema.u8, {})".to_string(), "a record of a type that is no record type"),
        (rules("extends = { chip = { weight = \"u8\" } },"), "an extension's type by name"),
        (rules("setup = { hp = \"u16\" },"), "a setup's type by name"),
    ] {
        let problems = checker.check(why, &format!("--!strict\n{bad}\n")).unwrap();
        assert!(!problems.is_empty(), "{why}: `{bad}` should not type-check");
    }
}

#[test]
fn misuse_of_the_core_api_is_a_type_error() {
    let mut checker = nettai_content_check::PackChecker::new(&nettai_content_check::definitions(&pack(), "exe6").unwrap()).unwrap();
    for (bad, why) in [
        ("local function f(me: Object) me.anmi = 3 end", "misspelled field"),
        ("local function f(me: Object) me:set_lifecycle(\"running\") end", "not a lifecycle state"),
        ("local function f(me: Object) me.sprite.shadow = \"soft\" end", "not a shadow"),
        ("local function f(me: Object) local _ = me.pos + 1 end", "Vec3 plus a number"),
        ("local _ = battle.spawn(\"projectile\", 3)", "not a pool"),
        ("local function f(me: Object) me.status.usingaction = true end", "not a status flag"),
        ("local function f(me: Object) local _ = me.held.x end", "not a button"),
        ("local function f(me: Object) me:set_status(\"guard\", true) end", "a status flag by a string"),
        ("local function f(me: Object) me.requests.gaurd = true end", "not a request"),
        ("local function f(me: Object) me.navi_flags.no_charging = true end", "not a navi flag"),
        ("local function f(me: Object) local _ = me.obstacle_flags.pushing end", "not an obstacle flag"),
        ("local function f(me: Object) local _ = me.form_traits.stands_on_magnets end", "not a form trait"),
        ("local function f(me: Object) me:console(0).palette = 1 end", "a sprite field on an object's console"),
        ("local function f(me: Object) me.sprite:console(0).white = true end", "a field every console draws alike"),
        ("local function f(me: Object, a: Action) me:set_attack(a, 2) end", "an attack kind by number"),
        ("local function f(me: Object) me:start_move(1) end", "a step's direction by number"),
        ("local function f(me: Object) local _: number = me:held_direction() end", "the held direction as a number"),
        ("local function f(me: Object) me.wrapped = 1 end", "wrapped as a number"),
        ("local function f(me: Object) me.charge_level = 2 end", "a charge level by number"),
        ("local function f(me: Object) local _ = me.collision.direction == 4 end", "a move's direction by number"),
        ("local function f(me: Object) local _ = me:take_damage(0) end", "a damage mode by number"),
        ("local function f(me: Object) obstacle.register(me, 0, 1) end", "an obstacle class by number"),
        ("local _ = battle.palette_flash(0, 4)", "a palette flash variant by number"),
        ("local _ = battle.cycle(1)", "a fight cycle by number"),
        ("battle.add_special_bonus(0, 1, 10)", "a special bonus by number"),
        ("custom.play(0, \"chips\")", "not a custom screen sound"),
        ("custom.fade(0, \"out\", 1)", "not a custom screen fade"),
        ("rules.listening(0).controler = false", "not a rules hook"),
        ("local _ = schema.role(\"form_lists\", \"u8\")", "not a role"),
        ("local function f(me: Object) me.element = 1 end", "an element by number"),
        ("local function f(me: Object) local _ = me.element + 1 end", "an element as a number"),
        ("local function f(me: Object) me.collision.secondary_element = 0x80 end", "the secondary elements' byte"),
        ("local function f(me: Object) local _ = me.collision.damage_elements == 0x20 end", "the secondary elements as bits"),
        ("local function f(me: Object) local _ = me.collision:element_damage(1) end", "a damage slot by number"),
        ("local function f(me: Object) me.collision:add_damage(\"heat\", 1, false) end", "not a primary element"),
        ("local _ = battle.weakness(1, 2)", "a weakness by element numbers"),
        ("local _ = Element.new(\"fire\", { \"sword\" })", "secondary elements as a list"),
        ("local function f(me: Object) me.sprite:set_flip(1) end", "the game's flip value"),
        ("local function f(me: Object) me.flip = 1 end", "a turn by number"),
        ("local function f(me: Object) battle.effect(me.pos, nil :: any, true) end", "an effect's facing as a boolean"),
        ("local function f(me: Object) me.sprite.hidden_parts = 0x80000000 end", "hidden parts as a word"),
        ("local function f(me: Object) me:set_header_flags(0x14) end", "header flags as a byte"),
        ("local function f(me: Object) local _ = me.collision.guard_dirs end", "the guard's directions as bits"),
        ("local _ = battle.navi(0).folder_1_regular + 1", "a folder entry that may be none"),
        ("local function f(me: Object) me:raise_barrier { behavior = \"plain\", hp = 1, threshold = 1, weak_element = 3 } end", "a barrier's weak element by number"),
        ("local _: TrapKind = \"anti_dmg\"", "not a trap"),
        ("local _: ChipTrait = \"no_chains\"", "not a chip trait"),
        ("local _: ChipClass = \"dark\"", "not a chip class"),
        ("local _: SetupProblemKind = \"too_many\"", "not a setup problem's kind"),
        // (A panel type is named, by its game's rules, so to the core a name is a string; one none of the game's
        // types has is refused as the battle runs. Its number is no name.)
        ("field.set_type(1, 1, 3)", "a panel type by number"),
        ("local function f(me: Object) me.status_timers.stun = 3 end", "not a status timer"),
        ("local _ = data.chips[1]", "not a data field"),
        ("local function f(me: Object) me.drag_step = \"sliding\" end", "not a drag step"),
        (
            "local function f(me: Object) battle.afterimage(me, me.pos, { anim = 0, facing = 0, color_shader = 0, lifetime = 1, shadow = \"soft\" }) end",
            "an afterimage's shadow that isn't one",
        ),
        ("local function f(me: Object) local _ = obstacle.react(me, \"shatters\") end", "not an obstacle crush"),
        ("local function f(me: Object) local _: \"gone\" = obstacle.removal(me) end", "not an obstacle removal"),
        ("local _: ProjectileShot = { kind = 0, damage = 1 }", "a shot without its height"),
        ("local function f(): SideSpecial return \"beast\" end", "not a side special"),
        (
            "local function f(me: Object) battle.form_overlay(me, { sprite = asset.sprite(\"x\"), stepping = \"dimmed\" }) end",
            "not a form overlay stepping",
        ),
        // Subtypes 8, 17, 18 (Wind, Anubis, Otenko) and the obstacle framework.
        ("local function f(me: Object) obstacle.take_hits(me, \"shoved\") end", "not an obstacle push"),
        ("local function f(me: Object) local _ = obstacle.react(me, \"breaks\", \"never\") end", "not a dimming hold"),
        // Dimming chip subtypes 2, 3, 5, 15 and 27.
        ("field.blink(1, 1, 3, 0)", "a blink to a panel type by number"),
        // Collision types and hit results by their members' names.
        ("local _ = new.collision { flags = { own_atack = true } }", "not a collision type member"),
        ("local _ = new.collision { flags = { own_attack = 1 } }", "a collision type member that isn't true or false"),
        ("local function f(me: Object) local _ = me.collision.hit_by.swrod end", "not a hit's name"),
        ("local _: HitNames = { swrod = true }", "not a hit's name in a set"),
        ("local _: Chip = { flags = { has_damag = true } }", "not a chip flag"),
        ("local _: FormDef = { sprite = asset.sprite(\"x\"), element = \"null\", buster_bonus = 0, traits = { erase = true } }", "not a form trait"),
        ("local function f(me: Object) me.collision.hit = 1 end", "a hit that isn't a boolean"),
        // The Gregar and Falzar chips.
        ("battle.hud().chips = false", "not a HUD field"),
        ("battle.hud(\"gauge\").gauge = false", "a HUD field for a side"),
        ("local _ = battle.hand_entry(0, 0).bonus", "not a hand entry's field"),
        ("local _: StatusDef = { duration = 90, timer = \"stun\" }", "not a status"),
        ("local function f(c: ChipSpec) c.extra_flags = { \"rush\" } end", "a chip's extra flags as a list"),
        ("local _: boolean = battle.effects[0x20000]", "a battle effect by its bit"),
    ] {
        let problems = checker.check(why, &format!("--!strict\n{bad}\n")).unwrap();
        assert!(!problems.is_empty(), "{why}: `{bad}` should not type-check");
    }
}

/// A check of some packs (a game and its support packs, as
/// `nettai-content publish` makes) answers for them alone: their type
/// errors and requires, and not another game's problems or a stray folder.
#[test]
fn a_check_of_some_packs_answers_for_them_alone() {
    let dir = std::env::temp_dir().join(format!("nettai-check-some-{}", std::process::id()));
    let write = |path: &str, text: &str| {
        let p = dir.join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    };
    write("nettai/core.d.luau", "declare level: number\n");
    write("g/manifest.toml", "id = \"g\"\nkind = \"game\"\ndepends = [\"lib\"]\n");
    write("g/init.luau", "local lib = require(\"@lib/x\")\nlocal n: number = level\nlocal s: string = level\nreturn {}\n");
    write("g/there.luau", "require(\"@self/gone\")\nreturn {}\n");
    write("lib/manifest.toml", "id = \"lib\"\nkind = \"support\"\n");
    write("lib/x.luau", "return {}\n");
    write("h/manifest.toml", "id = \"h\"\nkind = \"game\"\n");
    write("h/x.luau", "local s: string = 1\nreturn {}\n");
    write("stray/x.luau", "return {}\n");
    let (checked, problems) = nettai_content_check::check_packs(&dir, &["g", "lib"]).unwrap();
    assert_eq!(checked, 3, "g's two modules and lib's: {problems:#?}");
    assert!(problems.iter().any(|p| p.starts_with("g/init.luau:3:")), "g's type error: {problems:#?}");
    assert!(problems.iter().any(|p| p.contains("require(\"@self/gone\")")), "g's require of no module: {problems:#?}");
    assert!(!problems.iter().any(|p| p.contains("h/") || p.contains("stray")), "only g's and lib's: {problems:#?}");
    let (_, problems) = nettai_content_check::check_packs(&dir, &["k"]).unwrap();
    assert!(problems.iter().any(|p| p.contains("no pack k")), "{problems:#?}");
    // The whole content's check still answers for all of it.
    let (_, problems) = nettai_content_check::check_content(&dir).unwrap();
    assert!(problems.iter().any(|p| p.starts_with("h/")) && problems.iter().any(|p| p.contains("stray")), "{problems:#?}");
    std::fs::remove_dir_all(&dir).ok();
}

/// Each game's lock (depends.lock) names the commits its support packs'
/// submodules are at: a game goes with the exelib it was checked against.
/// (Where the support packs are no git checkouts, there's nothing to see.)
#[test]
fn the_games_go_with_the_support_packs_checked_out() {
    let (problems, notes) = nettai_content_check::locks(&pack(), None).unwrap();
    assert!(problems.is_empty(), "locks:\n{}", problems.join("\n"));
    if notes.is_empty() {
        assert!(nettai_content_api::packs::read_lock(&pack(), "exe6").unwrap().contains_key("exelib"), "exe6 locks exelib");
    }
}

/// A lock that names another commit than the support pack's checkout is
/// refused with both commits; a support pack that is no git checkout (a
/// bundle's folder) is a note, and its locks aren't judged.
#[test]
fn a_lock_on_another_commit_is_refused() {
    let dir = std::env::temp_dir().join(format!("nettai-check-lock-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let write = |path: &str, text: &str| {
        let p = dir.join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    };
    write("g/manifest.toml", "id = \"g\"\nkind = \"game\"\ndepends = [\"lib\"]\n");
    write("lib/manifest.toml", "id = \"lib\"\nkind = \"support\"\n");
    write("g/depends.lock", &format!("lib = \"{}\"\n", "a".repeat(40)));
    let (problems, notes) = nettai_content_check::locks(&dir, None).unwrap();
    assert!(problems.is_empty() && notes.len() == 1 && notes[0].starts_with("lib/: no git checkout"), "{problems:?} {notes:?}");
    let git = |args: &[&str]| assert!(std::process::Command::new("git").arg("-C").arg(dir.join("lib")).args(args).status().unwrap().success());
    git(&["init", "-q"]);
    git(&["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--allow-empty", "-m", "x"]);
    let head = String::from_utf8(std::process::Command::new("git").arg("-C").arg(dir.join("lib")).args(["rev-parse", "HEAD"]).output().unwrap().stdout).unwrap();
    let (problems, notes) = nettai_content_check::locks(&dir, None).unwrap();
    assert!(notes.is_empty(), "{notes:?}");
    assert!(
        problems.len() == 1 && problems[0].contains(&format!("locks lib at {}, and lib is at {}", "a".repeat(40), head.trim())),
        "{problems:?}"
    );
    write("g/depends.lock", &format!("lib = \"{}\"\n", head.trim()));
    assert_eq!(nettai_content_check::locks(&dir, None).unwrap(), (Vec::new(), Vec::new()));
    std::fs::remove_dir_all(&dir).ok();
}
