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
-- Not filled for want of a BN5 counterpart: the freeze and bubble sounds and sprites (BN5 has neither status), the
-- eruption's (BN5's lava panels don't erupt: they burn and go, §15), the Cross and Beast Out sounds and the custom
-- screen's Cross window, re-deal and scrap (BN6's).

local antidmg_counter = require("../chips/antidmg/counter")
local barriers = require("../lib/barriers")
local charged_shot = require("../navis/megaman/weapons/charged-shot")
local anti_recovery = require("../chips/antirecv/controller")
local soldier = require("../objects/soldier/soldier")
local chaos = require("./souls/chaos")
local drive_end = require("../chips/darkinvs/drive-end")
local dark_survival = require("./emotion/dark-survival")
local invalid = require("../chips/invalid/chip")
local collision = require("./collision")
local effects = require("@bn6/lib/effects")
local regions = require("@common/regions")
local sparks = require("../lib/sparks")
local statuses = require("./status")
local swords = require("../lib/swords")

return define.roles {
    id = "bn5:roles",
    actions = {
        -- The charged shot a navi's request starts from idle without its weapon's setup (BN5's 0x1E: the shared
        -- action, @common/megaman/charged_shot, navis/megaman/weapons/charged-shot).
        forced_charged_shot = charged_shot.action,
        -- The trap chips' counters, which a trap that caught a hit starts (the original's actions 0x47 and
        -- 0x48: AntiDmg's and AntiSwrd's, BN6's code; AntiSwrd's swinging BN5's blade and throwing BN5's sonic
        -- booms, lib/swords).
        anti_damage_counter = antidmg_counter.action,
        anti_sword_counter = swords.anti_sword_counter,
        -- A Chaos Unison charge released out of its window (BN5's action 0x39: ./souls/chaos): Dark MegaMan comes.
        chaos_failure = chaos.failure,
        -- The request 0x80000's action (BN5's 0x49): the end of DarkInvs's no-charge drive, which its timer's end asks
        -- for (rules/status's `no_charge_drive`; chips/darkinvs/drive-end). BN6's is its stun strike.
        stun_strike = drive_end.action,
        -- The request 0x40000000's action (BN5's 0x30): a dark MegaMan's last stand, which a loss of HP that brings
        -- him to 0 asks for (rules/status's `hp_loss`; ./emotion/dark-survival). BN6's is a volley no routine asks
        -- for.
        volley = dark_survival.action,
    },
    chips = {
        -- What a selection the custom screen can't allow becomes, and what a chip use the light and dark rule refuses
        -- becomes (BN5's chip 0x185, past the named ones: chips/invalid).
        invalid = invalid,
    },
    kinds = {
        -- AntiRecv's counterattack, which a heal meets when the other side has AntiRecv armed (BN6's code).
        anti_recovery = anti_recovery.kind,
        -- What an obstacle turns into where an armed side's ColonelSoul can use it (attack object 0x30: BN5's own,
        -- objects/soldier).
        obstacle_soldier = soldier.kind,
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
        -- BN5's hit sparks (lib/sparks: BN6's rows with BN5's sprites): a new registration's, a blocked hit's,
        -- a thrown obstacle's landing's.
        plain = sparks.plain,
        guard = sparks.guard,
        thrown_obstacle = sparks.plain,
        -- A lava panel's burn (the sparks' row 1, the fire spark: 0x08016D80).
        panel_burn = sparks.fire,
    },
    effects = {
        -- A navi deleted, HP recovered, the flash where a side cut in: BN5's effects 3, 6 and 0x1E are BN6's
        -- (0x080DFC94, BN6's `byte_80E0398`: the same rows).
        deletion = effects.deletion,
        recovery = effects.heal,
        cut_in_flash = effects.cut_in_flash,
        -- The mark over a navi whose trap sprang (AntiNavi, AntiRecv): BN5's effect 43 is BN6's 0x46 (hit
        -- sparks, animation 7), where rules/chip-use's `anti_navi_sparkle` puts it (BN5's on the panel's center,
        -- 16 pixels up; BN6's 16 pixels down the field and 32 up).
        trap_mark = effects.trap_mark,
        -- The splash of a body a sea panel holds at a move's end (effect 0x63: 0x0801715E).
        panel_splash = define.effect { sprite = asset.sprite("bn5:splash"), anim = 0, palette = 0 },
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
        panel_crack = asset.sound("bn5:panel-crack"),
        panel_poison = asset.sound("bn5:panel-poison"),
        -- A navi hit, as its own player hears it and as the other does; an object taking damage; a
        -- blocked hit; a counter hit; a navi deleted.
        own_hit = asset.sound("bn5:own-hit"),
        hit = asset.sound("bn5:hit"),
        damage = asset.sound("bn5:damage"),
        guard = asset.sound("bn5:guard-hit"),
        counter_hit = asset.sound("bn5:counter-hit"),
        deleted = asset.sound("bn5:deleted"),
        -- HP recovered; a chip's damage bonus as it is used.
        recovery = asset.sound("bn5:heal"),
        damage_bonus = asset.sound("bn5:damage-bonus"),
        -- The battle pauses and resumes; the custom gauge fills; the low-HP alarm.
        pause = asset.sound("bn5:pause"),
        gauge_full = asset.sound("bn5:gauge-full"),
        low_hp = asset.sound("bn5:low-hp"),
        -- A side cuts in, a trap springs; a telop shows.
        cut_in = asset.sound("bn5:dimming-sparkle"),
        telop = asset.sound("bn5:telop"),
        -- The buster starts charging, and is charged.
        buster_charge = asset.sound("bn5:buster-charge"),
        buster_charged = asset.sound("bn5:buster-charged"),
        -- Confusion's stars.
        confusion = asset.sound("bn5:confusion"),
        -- A navi turns invisible, shows again (or enters), arrives mid-battle; something goes.
        invisible = asset.sound("bn5:invisible"),
        appear = asset.sound("bn5:appear"),
        arrive = asset.sound("bn5:point-appear"),
        fade = asset.sound("bn5:fade"),
        -- A thrown obstacle lifted, and flying.
        obstacle_lift = asset.sound("bn5:point-rise"),
        obstacle_throw = asset.sound("bn5:etomahwk"),
        -- SELECT with too little gauge, and what can't be picked or taken back on the custom screen.
        refused = asset.sound("bn5:cant-jack-in"),
        -- The custom screen's: the window slides in; the cursor moves; SELECT hides and shows the window; a
        -- pick, OK, a pick taken back; L's no-running message; R's description opens and closes; the
        -- Program Advance animation's chips of the recipe and the Program Advance.
        custom_open = asset.sound("bn5:select-79"),
        custom_cursor = asset.sound("bn5:menu-cur-move"),
        custom_hide = asset.sound("bn5:cur-move-80"),
        custom_pick = asset.sound("bn5:menu-select"),
        custom_ok = asset.sound("bn5:select-82"),
        custom_back = asset.sound("bn5:exit-submenu"),
        custom_run_message = asset.sound("bn5:unselect-7b"),
        custom_description = asset.sound("bn5:description-open"),
        custom_description_close = asset.sound("bn5:description-close"),
        program_advance_part = asset.sound("bn5:err-select-91"),
        program_advance = asset.sound("bn5:delta-ray-2"),
    },
    music = {
        -- A link battle's, whatever its stage's.
        link_battle = asset.sound("bn5:virus-battle"),
        -- The winner's (in special battles, and otherwise) and the loser's.
        winner_special = asset.sound("bn5:winner-0"),
        winner = asset.sound("bn5:winner-1"),
        loser = asset.sound("bn5:loser"),
    },
    sprites = {
        -- The charge glow of a B charge; of an A charge (14-15); of an armed Chaos Unison charge (14-1C), its
        -- animation the chaos cycle's window (0x080E0EA4).
        charge_glow = asset.sprite("bn5:charge-glow"),
        charge_glow_a = asset.sprite("bn5:charge-glow-a"),
        charge_glow_chaos = asset.sprite("bn5:chaos-charge-glow"),
        -- The status visuals over a navi (0x080E08EC, BN6's `byte_80E08E4`): confusion's stars and blindness (BN6's
        -- numbers) and the immobilized one (10-2d, BN6's 10-00).
        confusion = asset.sprite("bn5:confusion"),
        blindness = asset.sprite("bn5:blindness"),
        immobilized = asset.sprite("bn5:immobilized"),
        -- The Full Synchro aura (actor object 0x5E, 0x080C4608: 14-16, BN6's sprite numbers; its animation the navi's
        -- actor record's AI index).
        full_synchro_aura = asset.sprite("bn5:full-synchro-aura"),
    },
    hooks = {
        -- The first barrier: the navi enters with the barrier its stats name (FstBarr's, BN6's code), and its
        -- visual (BN5's, lib/barriers).
        first_barrier = function(navi: Object)
            local barrier = battle.navi(navi.alliance).first_barrier
            if barrier then
                barriers.raise(navi, barrier, true)
            end
        end,
    },
    banners = {
        -- A round's start (the battle's number), a turn's, the final turns'.
        round_start = asset.banner("bn5:battle-number-start"),
        turn_start = asset.banner("bn5:turn-start"),
        final_turn = asset.banner("bn5:final-turn"),
        -- A draw; the damage judge's.
        draw = asset.banner("bn5:draw"),
        judge = asset.banner("bn5:hit-damage-judge"),
        -- A telop on the local player's side, and on the other's.
        telop = asset.banner("bn5:telop"),
        telop_remote = asset.banner("bn5:telop-remote"),
        -- A Program Advance, and a selection that makes none.
        program_advance = asset.banner("bn5:program-advance"),
        program_advance_empty = asset.banner("bn5:program-advance-empty"),
    },
}
