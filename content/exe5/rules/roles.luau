--!strict
-- What EXE5's rules start, spawn, show and play by role (docs/design/content-model-v2.md §7.4,
-- docs/design/rules-in-luau.md R2: a game's roles are its root's). So far the roles that name only
-- assets and data EXE5 has: the sounds, music, banners and sprites its pack names (content/exe5/compat/
-- assets.toml: EXE6's names where the asset or its place in the code is EXE6's), its collision types, the
-- anchor region (EXE6's, as EXE5's region 0x01 is), and the traps' counters and mark (EXE6's, as EXE5's trap
-- chips are EXE6's code). The other roles that name actions, kinds, chips, statuses, effects and sparks wait
-- for the port's EXE5 definitions (docs/design/exe5-map.md §15); the content check lists them as unfilled,
-- and a battle stops at the first it needs.
--
-- Not filled for want of an EXE5 counterpart: the freeze and bubble sounds and sprites (EXE5 has neither status), the
-- eruption's (EXE5's lava panels don't erupt: they burn and go, §15), the Cross and Beast Out sounds and the custom
-- screen's Cross window, re-deal and scrap (EXE6's).

local antidmg_counter = require("../chips/antidmg/counter")
local barriers = require("../lib/barriers")
local beat = require("../chips/beat").beat
local charged_shot = require("../navis/megaman/weapons/charged_shot")
local anti_recovery = require("../chips/antirecv/controller")
local dive_ripple = require("../objects/dive_ripple/ripple")
local drive_end = require("../chips/darkinvs/drive_end")
local dark_survival = require("./emotion/dark_survival")
local invalid = require("../chips/invalid").invalid
local collision = require("./collision")
local effects = require("../lib/effects")
local regions = require("@exelib/regions")
local rush = require("../chips/rush").rush
local sparks = require("../lib/sparks")
local statuses = require("./status")
local supports = require("../lib/supports/controller")
local swords = require("../lib/swords")
local tango = require("../chips/tango").tango

local roles: RolesSpec = {
    actions = {
        -- The charged shot a navi's request starts from idle without its weapon's setup (EXE5's 0x1E: the shared
        -- action, @exelib/megaman/charged_shot, navis/megaman/weapons/charged_shot).
        forced_charged_shot = charged_shot.action,
        -- The trap chips' counters, which a trap that caught a hit starts (the original's actions 0x47 and
        -- 0x48: AntiDmg's and AntiSwrd's, EXE6's code; AntiSwrd's swinging EXE5's blade and throwing EXE5's sonic
        -- booms, lib/swords).
        anti_damage_counter = antidmg_counter.action,
        anti_sword_counter = swords.anti_sword_counter,
        -- The request 0x80000's action (EXE5's 0x49): the end of DarkInvs's no-charge drive, which its timer's end asks
        -- for (rules/status's `no_charge_drive`; chips/darkinvs/drive_end). EXE6's is its stun strike.
        stun_strike = drive_end.action,
        -- The request 0x40000000's action (EXE5's 0x30): a dark MegaMan's last stand, which a loss of HP that brings
        -- him to 0 asks for (the rules' `hp_emptied`; ./emotion/dark_survival). EXE6's is a volley no routine asks
        -- for.
        volley = dark_survival.action,
    },
    chips = {
        -- What a selection the custom screen can't allow becomes, and what a chip use the light and dark rule refuses
        -- becomes (EXE5's chip 0x185, past the named ones: chips/invalid).
        invalid = invalid,
        -- The chips the NaviCust supports' telops name (EXE5's 0x179 to 0x17B, past the library: EXE6's numbers).
        rush = rush,
        beat = beat,
        tango = tango,
    },
    kinds = {
        -- AntiRecv's counterattack, which a heal meets when the other side has AntiRecv armed (EXE6's code).
        anti_recovery = anti_recovery.kind,
        -- The NaviCust supports' controller (Rush's, Beat's and Tango's: EXE5's effect object 0x74, EXE6's code,
        -- lib/supports).
        support = supports.kind,
        -- The ripple over a body under the sea's surface (effect object 0x3E: objects/dive_ripple).
        dive_ripple = dive_ripple.kind,
    },
    statuses = {
        -- What a damage word's paralysis bit makes its hits carry, and what a counter hit lands instead of a
        -- flinch (EXE5's status bytes 0x10 and 0x12, EXE6's numbers).
        damage_word_paralysis = statuses.paralyze_90,
        -- What its confusion and blindness bits (0x2000, 0x1000) make them carry (0x080165EC: bytes 0x20, 0x30).
        damage_word_confusion = statuses.confuse_480,
        damage_word_blindness = statuses.blind_480,
        counter_paralysis = statuses.paralyze_150,
    },
    sparks = {
        -- EXE5's hit sparks (lib/sparks: EXE6's rows with EXE5's sprites): a new registration's, a blocked hit's,
        -- a thrown obstacle's landing's.
        plain = sparks.plain,
        guard = sparks.guard,
        thrown_obstacle = sparks.plain,
    },
    effects = {
        -- A navi deleted, HP recovered, the flash where a side cut in: EXE5's effects 3, 6 and 0x1E are EXE6's
        -- (0x080DFC94, EXE6's `byte_80E0398`: the same rows).
        deletion = effects.deletion,
        recovery = effects.heal,
        cut_in_flash = effects.cut_in_flash,
        -- The mark over a navi whose trap sprang (AntiNavi, AntiRecv): EXE5's effect 43 is EXE6's 0x46 (hit
        -- sparks, animation 7), where rules/chip_use's `anti_navi_sparkle` puts it (EXE5's on the panel's center,
        -- 16 pixels up; EXE6's 16 pixels down the field and 32 up).
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
        -- SearchSoul's Shuffle's shuffles (0x08023512: the framework's re-deal, each of its eight steps).
        custom_redeal_shuffle = asset.sound("hop"),
        -- The custom screen's: the window slides in; the cursor moves; SELECT hides and shows the window; a
        -- pick, OK, a pick taken back; L's no-running message; R's description opens and closes; the
        -- Program Advance animation's chips of the recipe and the Program Advance.
        custom_open = asset.sound("select-79"),
        custom_cursor = asset.sound("menu-cur-move"),
        custom_hide = asset.sound("cur-move-80"),
        -- The hover over a dark chip, every 64 ticks while the screen is dark or fading (0x08025AA2: sound 0x16B).
        custom_shade = asset.sound("dark-hover"),
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
        -- The charge glow of a B charge; of an A charge (14-15). (An armed Chaos Unison charge's, 14-1C, is
        -- ./souls/chaos_charge's.)
        charge_glow = asset.sprite("charge-glow"),
        charge_glow_a = asset.sprite("charge-glow-a"),
        -- The status visuals over a navi (0x080E08EC, EXE6's `byte_80E08E4`): confusion's stars and blindness (EXE6's
        -- numbers) and the immobilized one (10-2d, EXE6's 10-00).
        confusion = asset.sprite("confusion"),
        blindness = asset.sprite("blindness"),
        immobilized = asset.sprite("immobilized"),
        -- The mark over a navi a damaging weakness hit landed on ("!!": effect object 0x6B, 0x080E84B0, EXE6's code
        -- with its animation 0 alone; spawned by 0x08017254).
        hit_marker = asset.sprite("hit-marker"),
        -- The Full Synchro aura (actor object 0x5E, 0x080C4608: 14-16, EXE6's sprite numbers; its animation the navi's
        -- actor record's AI index).
        full_synchro_aura = asset.sprite("full-synchro-aura"),
    },
    hooks = {
        -- The first barrier: the navi enters with the barrier its stats name (FstBarr's, EXE6's code), and its
        -- visual (EXE5's, lib/barriers).
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
        -- A draw; a win's where it isn't the winner's navi's (every battle but an operation battle:
        -- 0x080074D2), and a win and a loss on the judge's ruling (0x0800758A); the damage judge's.
        draw = asset.banner("draw"),
        win = asset.banner("enemy-deleted"),
        win_judged = asset.banner("you-win"),
        lose_judged = asset.banner("you-lose"),
        judge = asset.banner("hit-damage-judge"),
        -- A telop on the local player's side, and on the other's.
        telop = asset.banner("telop"),
        telop_remote = asset.banner("telop-remote"),
        -- A Program Advance, and a selection that makes none.
        program_advance = asset.banner("program-advance"),
        program_advance_empty = asset.banner("program-advance-empty"),
    },
}

return roles
