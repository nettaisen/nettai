//! The content (content/) type-checks, each pack against its
//! declarations; its manifests and requires name only modules that are
//! there, each require one its pack may make; and misuse of the API is a
//! type error.

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

/// docs/design/content-model-v2.md §4.0: a listed module or a require of
/// no module, a folder that is no pack, a require of another game's module
/// (game to game, support to game) and a cycle of support packs fail the
/// check with their paths.
#[test]
fn what_the_packs_refuse() {
    let dir = std::env::temp_dir().join(format!("nettai-check-{}", std::process::id()));
    let write = |path: &str, text: &str| {
        let p = dir.join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    };
    write("g/manifest.toml", "id = \"g\"\nkind = \"game\"\nuses = [\"lib\"]\n[definitions]\nrules = [\"there\", \"gone\"]\n");
    write("g/there.luau", "local y = require(\"./nowhere\")\nlocal z = require(\"@h/x\")\nlocal w = require(\"@lib/x\")\nreturn {}\n");
    write("h/manifest.toml", "id = \"h\"\nkind = \"game\"\n");
    write("h/x.luau", "return {}\n");
    write("lib/manifest.toml", "id = \"lib\"\nkind = \"support\"\nuses = [\"base\"]\n");
    write("lib/x.luau", "local g = require(\"@g/there\")\nlocal _ = asset.sprite(\"g:x\")\nreturn {}\n");
    write("base/manifest.toml", "id = \"base\"\nkind = \"support\"\nuses = [\"lib\"]\n");
    write("stray/x.luau", "return {}\n");
    let problems = nettai_content_check::reach(&dir).unwrap();
    let has = |want: &str| assert!(problems.iter().any(|p| p.contains(want)), "{want}: {problems:#?}");
    has("g/manifest.toml: lists gone, and no module g/gone.luau is there");
    has("g/there.luau: require(\"./nowhere\"): no module g/nowhere.luau");
    has("g/there.luau: require(\"@h/x\"): h is a game pack, which no other pack requires");
    has("lib/x.luau: require(\"@g/there\"): g is a game pack, which no other pack requires");
    has("lib/x.luau: support pack lib names an asset");
    has("its uses make a cycle: base uses lib uses base");
    has("stray/: no manifest.toml; a folder of content/ is a pack");
    std::fs::remove_dir_all(&dir).ok();
}

/// The engine's test pack and test content (crates/nettai-battle/testdata)
/// type-check against the same definitions, and pass the lints.
#[test]
fn the_test_pack_and_test_content_type_check() {
    let mut checker = nettai_content_check::PackChecker::new(&nettai_content_check::definitions(&pack(), "bn6").unwrap()).unwrap();
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

#[test]
fn misuse_of_the_v2_api_is_a_type_error() {
    let mut checker = nettai_content_check::PackChecker::new(&nettai_content_check::definitions(&pack(), "bn6").unwrap()).unwrap();
    for (bad, why) in [
        ("local _ = define.kind { id = 'x', pool = 'water', update = function(me: Object) end }", "not a pool"),
        ("local _ = define.kind { id = 'x', pool = 'attack' }", "a kind without its update"),
        ("local _ = asset.sprite(3)", "an asset by number"),
        ("local _ = define.effect { sprite = 'bomb' }", "a sprite by string"),
        ("local function f(me: Object) me:set_attack('shot', 1) end", "an action by name"),
        ("local function f(me: Object) local _ = battle.spawn(me, me.pos) end", "an object for a kind"),
        ("local _ = define.region { panels = { 'front' } }", "a panel that isn't { dx, dy }"),
        ("local _ = define.roles { actions = { anti_damage_counter = 3 } }", "a role that isn't an action"),
    ] {
        let problems = checker.check(why, &format!("--!strict\n{bad}\n")).unwrap();
        assert!(!problems.is_empty(), "{why}: `{bad}` should not type-check");
    }
}

#[test]
fn the_numeric_api_that_is_gone_is_a_type_error() {
    let mut checker = nettai_content_check::PackChecker::new(&nettai_content_check::definitions(&pack(), "bn6").unwrap()).unwrap();
    for (bad, why) in [
        ("local _ = battle.spawn(\"attack\", 8)", "a spawn by pool and index"),
        ("local _ = battle.spawn_kind(\"rock\")", "a spawn by name"),
        ("local function f(me: Object) local _ = me:param(1) end", "a spawn parameter"),
        ("local function f(me: Object) local _ = me:attack_param(1) end", "an attack parameter"),
        ("local function f(me: Object) local _ = me.index end", "a kind by index"),
        ("local function f(me: Object) local _ = me.variant end", "the attack's variant"),
        ("local function f(me: Object) local _: number = me.chip end", "the attack's chip by number"),
        ("local function f(me: Object) me:set_attack(0x12, 2) end", "an action by number"),
        ("local function f(me: Object) me.sprite:load(\"0c-01\") end", "a sprite by number"),
        ("local _ = battle.hand_chip(0, 0)", "a hand's chip by number"),
        ("local function f(me: Object) dimming.show_navi_telop(me, 0x123) end", "a telop's chip by number"),
        ("battle.set_linked(0, { chip = 0x123, bonus = 0, damage = 0 })", "a linked record's chip by number"),
        ("local _ = define.roles { actions = { turn = { legacy = { action = 0x3B } } } }", "a role by action number"),
        ("local _ = define.roles { kinds = { support = { legacy = { kind = \"support\" } } } }", "a role by kind key"),
        ("local _ = data.rules.sine[1]", "a rule table by number"),
        ("local function f(me: Object) local _ = battle.effect(me.pos, 3) end", "an effect by number"),
        ("battle.play_sound(0x10)", "a sound by number"),
        ("local function f(me: Object) local _ = me.name_id end", "an object's NameID"),
        ("local _ = battle.navi_record(0x1A0)", "an actor record by NameID"),
        ("local function f(me: Object) me:death_hook(0x1A0) end", "a death hook by NameID"),
        ("local _ = battle.attach_point(0x1A0, 3, 0, 0)", "an attach point by NameID"),
        ("local function f(me: Object) me:setup_collision(4, 5, 0) end", "collision types by number"),
    ] {
        let problems = checker.check(why, &format!("--!strict\n{bad}\n")).unwrap();
        assert!(!problems.is_empty(), "{why}: `{bad}` should not type-check");
    }
}

#[test]
fn misuse_of_the_core_api_is_a_type_error() {
    let mut checker = nettai_content_check::PackChecker::new(&nettai_content_check::definitions(&pack(), "bn6").unwrap()).unwrap();
    for (bad, why) in [
        ("local function f(me: Object) me.anmi = 3 end", "misspelled field"),
        ("local function f(me: Object) me:set_lifecycle(\"running\") end", "not a lifecycle state"),
        ("local function f(me: Object) me.sprite.shadow = \"soft\" end", "not a shadow"),
        ("local function f(me: Object) local _ = me.pos + 1 end", "Vec3 plus a number"),
        ("local _ = battle.spawn(\"projectile\", 3)", "not a pool"),
        ("local function f(me: Object) me:set_status(\"usingaction\", true) end", "not a status flag"),
        ("local function f(me: Object) local _ = me:held(\"x\") end", "not a button"),
        ("field.set_type(1, 1, \"magma\")", "not a panel type"),
        ("local function f(me: Object) me:set_status_timer(\"stun\", 3) end", "not a status timer"),
        ("local _ = data.chips[1]", "not a data field"),
        ("local function f(me: Object) me.drag_step = \"sliding\" end", "not a drag step"),
        (
            "local function f(me: Object) battle.afterimage(me, me.pos, { anim = 0, flip = 0, color_shader = 0, lifetime = 1, shadow = \"soft\" }) end",
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
        ("local function f(me: Object) battle.set_wind(me, 0, \"chip\") end", "not a wind source"),
        // Dimming chip subtypes 2, 3, 5, 15 and 27.
        ("field.blink(1, 1, \"magma\", 0)", "a blink to not a panel type"),
        // The Gregar and Falzar chips.
        ("battle.show_hud({ \"chips\" }, false)", "not a HUD part"),
        ("battle.show_hud(\"gauge\", false)", "a HUD part, not a list of them"),
    ] {
        let problems = checker.check(why, &format!("--!strict\n{bad}\n")).unwrap();
        assert!(!problems.is_empty(), "{why}: `{bad}` should not type-check");
    }
}
