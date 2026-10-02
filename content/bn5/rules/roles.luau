--!strict
-- What BN5's rules start, spawn, show and play by role (docs/design/content-model-v2.md §7.4,
-- docs/design/rules-in-luau.md R2: a game's roles are its root's). So far the roles that name only
-- assets and data BN5 has: the sounds, music, banners and sprites its pack names (content/bn5/compat/
-- assets.toml: BN6's names where the asset or its place in the code is BN6's), its collision types, the
-- anchor region (BN6's, as BN5's region 0x01 is), and the traps' counters and mark (BN6's, as BN5's trap
-- chips are BN6's code). The other roles that name actions, kinds, chips, statuses, effects and sparks wait
-- for the port's BN5 definitions (docs/design/bn5-map.md §15); the content check lists them as unfilled,
-- and a battle stops at the first it needs.
--
-- Not filled for want of a BN5 counterpart: the status visuals' sprites (BN5's sprites differ from
-- BN6's; named once the port draws them), the A charge's glow and Full Synchro's aura (BN6's), the
-- freeze and bubble sounds and sprites (BN5 has neither status), the eruption's (BN5's lava panels
-- don't erupt: they burn and go, §15), the Cross and Beast Out sounds and the custom screen's Cross
-- window, re-deal and scrap (BN6's).

local antidmg_counter = require("@bn6/chips/antidmg/counter")
local barriers = require("@bn6/lib/barriers/barriers")
local charged_shot = require("@bn6/navis/megaman/weapons/charged-shot/weapon")
local anti_recovery = require("@bn6/chips/antirecv/controller")
local antiswrd_counter = require("@bn6/chips/antiswrd/counter")
local collision = require("./collision")
local effects = require("@bn6/lib/effects")
local regions = require("@bn6/lib/regions")
local sparks = require("@bn6/lib/sparks")
local statuses = require("./status")

return define.roles {
    actions = {
        -- The charged shot a navi's request starts from idle without its weapon's setup (BN6's action, BN5's
        -- 0x1E: the same code but for the flash's attachment row).
        forced_charged_shot = charged_shot.action,
        -- The trap chips' counters, which a trap that caught a hit starts (the original's actions 0x47 and
        -- 0x48: AntiDmg's and AntiSwrd's, BN6's code).
        anti_damage_counter = antidmg_counter.action,
        anti_sword_counter = antiswrd_counter.action,
    },
    kinds = {
        -- AntiRecv's counterattack, which a heal meets when the other side has AntiRecv armed (BN6's code).
        anti_recovery = anti_recovery.kind,
    },
    statuses = {
        -- What a damage word's paralysis bit makes its hits carry, and what a counter hit lands instead of a
        -- flinch (BN5's status bytes 0x10 and 0x12, BN6's numbers).
        damage_word_paralysis = statuses.paralyze_90,
        counter_paralysis = statuses.paralyze_150,
        -- The NaviCust on-hit bug's, by its level (0x32, 0x22).
        hit_bug_blind = statuses.blind_1200,
        hit_bug_confuse = statuses.confuse_960,
    },
    sparks = {
        -- BN5's hit sparks 0 to 0xD are BN6's (0x080E0814, BN6's `off_80E0804`: the same rows): a new
        -- registration's, a blocked hit's, a thrown obstacle's landing's.
        plain = sparks.plain,
        guard = sparks.guard,
        thrown_obstacle = sparks.charged,
    },
    effects = {
        -- A navi deleted, HP recovered, the flash where a side cut in: BN5's effects 3, 6 and 0x1E are BN6's
        -- (0x080DFC94, BN6's `byte_80E0398`: the same rows).
        deletion = effects.deletion,
        recovery = effects.heal,
        cut_in_flash = effects.cut_in_flash,
        -- The mark over a navi whose trap sprang (AntiNavi, AntiRecv): BN5's effect 43 is BN6's 0x46 (hit
        -- sparks, animation 7). (BN5 shows it on the panel's center, 16 pixels up; BN6 16 pixels down the
        -- field and 32 up: the engine's, §15.3.)
        trap_mark = effects.trap_mark,
    },
    collision = {
        -- A navi's body, its body when it floats, and what it reacts to.
        navi = collision.navi,
        floating_navi = collision.floating_navi,
        navi_target = collision.navi_target,
        -- A thrown obstacle's landing, and what it reaches.
        thrown_obstacle = collision.breaking,
        thrown_obstacle_target = collision.hits_navis,
    },
    regions = {
        -- A registration's own panel, which every setup gives it.
        anchor = regions.single,
    },
    sounds = {
        -- A panel cracks or breaks; a panel turns to poison.
        panel_crack = asset.sound("panel-crack"),
        panel_poison = asset.sound("panel-poison"),
        -- A navi hit, as its own player hears it and as the other does; an object taking damage; a
        -- blocked hit; a counter hit; a navi deleted.
        own_hit = asset.sound("own-hit"),
        hit = asset.sound("hit"),
        damage = asset.sound("damage"),
        guard = asset.sound("guard-hit"),
        counter_hit = asset.sound("counter-hit"),
        deleted = asset.sound("deleted"),
        -- HP recovered; a chip's damage bonus as it is used.
        recovery = asset.sound("heal"),
        damage_bonus = asset.sound("damage-bonus"),
        -- The battle pauses and resumes; the custom gauge fills; the low-HP alarm.
        pause = asset.sound("pause"),
        gauge_full = asset.sound("gauge-full"),
        low_hp = asset.sound("low-hp"),
        -- A side cuts in, a trap springs; a telop shows.
        cut_in = asset.sound("dimming-sparkle"),
        telop = asset.sound("telop"),
        -- The buster starts charging, and is charged.
        buster_charge = asset.sound("buster-charge"),
        buster_charged = asset.sound("buster-charged"),
        -- Confusion's stars.
        confusion = asset.sound("confusion"),
        -- A navi turns invisible, shows again (or enters), arrives mid-battle; something goes.
        invisible = asset.sound("invisible"),
        appear = asset.sound("appear"),
        arrive = asset.sound("point-appear"),
        fade = asset.sound("fade"),
        -- A thrown obstacle lifted, and flying.
        obstacle_lift = asset.sound("point-rise"),
        obstacle_throw = asset.sound("etomahwk"),
        -- SELECT with too little gauge, and what can't be picked or taken back on the custom screen.
        refused = asset.sound("cant-jack-in"),
        -- The custom screen's: the window slides in; the cursor moves; SELECT hides and shows the window; a
        -- pick, OK, a pick taken back; L's no-running message; R's description opens and closes; the
        -- Program Advance animation's chips of the recipe and the Program Advance.
        custom_open = asset.sound("select-79"),
        custom_cursor = asset.sound("menu-cur-move"),
        custom_hide = asset.sound("cur-move-80"),
        custom_pick = asset.sound("menu-select"),
        custom_ok = asset.sound("select-82"),
        custom_back = asset.sound("exit-submenu"),
        custom_run_message = asset.sound("unselect-7b"),
        custom_description = asset.sound("description-open"),
        custom_description_close = asset.sound("description-close"),
        program_advance_part = asset.sound("err-select-91"),
        program_advance = asset.sound("delta-ray-2"),
    },
    music = {
        -- A link battle's, whatever its stage's.
        link_battle = asset.sound("virus-battle"),
        -- The winner's (in special battles, and otherwise) and the loser's.
        winner_special = asset.sound("winner-0"),
        winner = asset.sound("winner-1"),
        loser = asset.sound("loser"),
    },
    sprites = {
        -- The charge glow of a B charge.
        charge_glow = asset.sprite("charge-glow"),
    },
    hooks = {
        -- The first barrier: the navi enters with the barrier its stats name (FstBarr's, BN6's code), and its
        -- visual.
        first_barrier = function(navi: Object)
            local barrier = battle.navi(navi.alliance).first_barrier
            if barrier then
                barriers.raise(navi, barrier, true)
            end
        end,
    },
    banners = {
        -- A round's start (the battle's number), a turn's, the final turns'.
        round_start = asset.banner("battle-number-start"),
        turn_start = asset.banner("turn-start"),
        final_turn = asset.banner("final-turn"),
        -- A draw; the damage judge's.
        draw = asset.banner("draw"),
        judge = asset.banner("hit-damage-judge"),
        -- A telop on the local player's side, and on the other's.
        telop = asset.banner("telop"),
        telop_remote = asset.banner("telop-remote"),
        -- A Program Advance, and a selection that makes none.
        program_advance = asset.banner("program-advance"),
        program_advance_empty = asset.banner("program-advance-empty"),
    },
}
