--!strict
-- What the ruleset starts and spawns by role (docs/design/content-model-v2.md
-- §7.4). A role whose target is still a v1 registration names it through
-- the transitional `legacy` marker (an action by its number, a kind by its
-- key), counted by the ratchet; a family that converts the target names its
-- definition here instead.

local absorbed_obstacle = require("../objects/absorbed-obstacle/absorbed_obstacle")
local anti_recovery = require("../chips/antirecv/controller")
local antidmg_counter = require("../chips/antidmg/counter")
local antiswrd_counter = require("../chips/antiswrd/counter")
local barriers = require("../lib/barriers/barriers")
local beast_out = require("../chips/beastout/chip")
local beat = require("../chips/beat/chip")
local beast_claw = require("../navis/megaman/weapons/beast-claw/weapon")
local bodygrd_counter = require("../chips/bodygrd/counter")
local charged_shot = require("../navis/megaman/weapons/charged-shot/weapon")
local collision = require("./collision")
local dust_beast_scatter = require("../navis/megaman/forms/dustcross-beast/scatter")
local effects = require("../lib/effects")
local encased_bubble = require("../objects/encased-bubble/bubble")
local invalid = require("../chips/invalid/chip")
local megabstr = require("../chips/megabstr/chip")
local rush = require("../chips/rush/chip")
local tango = require("../chips/tango/chip")
local falling_rock = require("../objects/falling-rock/falling_rock")
local ground_drill = require("../navis/megaman/forms/groundcross/drill")
local lockon = require("./lockon")
local regions = require("../lib/regions")
local slash_beast_lunge = require("../navis/megaman/forms/slashcross-beast/lunge")
local slash_charge = require("../navis/megaman/forms/slashcross/charge")
local sparks = require("../lib/sparks")
local stun_strike = require("../lib/swords/stun_strike")
local statuses = require("./status")
local supports = require("../lib/supports/controller")

return define.roles {
    actions = {
        -- The charged shot a navi's request starts from idle without its
        -- weapon's setup (the original's action 0x16).
        forced_charged_shot = charged_shot.action,
        -- The trap chips' counters, which a trap that caught a hit starts
        -- (the original's actions 0x47, 0x48 and 0x4B; reactive.rs).
        anti_damage_counter = antidmg_counter.action,
        anti_sword_counter = antiswrd_counter.action,
        body_guard_counter = bodygrd_counter.action,
        -- The strike the navi's request 0x80000 starts from idle (the
        -- machine swords' strike by the attack's variant).
        stun_strike = stun_strike.action,
        -- What a Cross navi protected at the battle's end runs
        -- (GroundCross's drill).
        cross_protect = ground_drill.action,
        -- The turn L or R starts where turning is enabled.
        turn = { legacy = { action = 0x3B } },
        -- A Cross navi's knock-out (SlashCross Beast's lunge).
        cross_death = slash_beast_lunge.action,
        -- A Cross navi's volley.
        volley = { legacy = { action = 0x30 } },
        -- SlashCross's charged slash, which the Beast rush recognizes.
        charged_sword = slash_charge.action,
        -- The Beast forms' charged claw.
        beast_claw = beast_claw.action,
        -- DustCross Beast's scatter, during which the navi isn't grounded.
        dust_beast_scatter = dust_beast_scatter.action,
    },
    kinds = {
        -- What an obstacle absorbed flies off as.
        absorbed_obstacle = absorbed_obstacle.kind,
        -- GroundCross's charged chips' falling rocks.
        falling_rock = falling_rock.kind,
        -- The NaviCust supports' controller (Rush's, Beat's and Tango's).
        support = supports.kind,
        -- AntiRecv's counterattack, which a heal meets when the other side
        -- has AntiRecv armed.
        anti_recovery = anti_recovery.kind,
    },
    chips = {
        -- What a zeroed chip field reads (the original's chip 0): an empty
        -- hand's chip, and a chip use with none.
        zeroed = megabstr,
        -- The custom screen's Beast Out button, as a chip in the hand.
        beast_out = beast_out,
        -- What a selection the custom screen can't allow becomes.
        invalid = invalid,
        -- The chips the supports' telops name.
        rush = rush,
        beat = beat,
        tango = tango,
    },
    statuses = {
        -- What a damage word's paralysis bit (0x4000) makes its hits carry.
        damage_word_paralysis = statuses.paralyze_90,
        -- What a counter hit lands instead of a flinch.
        counter_paralysis = statuses.paralyze_150,
        -- What an aqua hit gives a body standing on ice.
        ice_freeze = statuses.freeze_150,
        -- The NaviCust on-hit bug's, by its level.
        hit_bug_blind = statuses.blind_1200,
        hit_bug_confuse = statuses.confuse_960,
    },
    lockon = {
        -- The Beast forms' charged claw's lock-on mode, whatever chip its
        -- attack carries.
        beast_claw = lockon.beast_claw,
    },
    effects = {
        -- A navi deleted (its two explosions), a cross merged into MegaMan,
        -- a navi arriving mid-battle or back from a Cross special, a form
        -- put on or taken off.
        deletion = effects.deletion,
        -- HP recovered: a heal's sparkle, a recovery chip's, the
        -- undershirt's credits'; a navi appearing mid-battle.
        recovery = effects.heal,
        -- The flash where a side cut in.
        cut_in_flash = effects.bat_impact,
        -- The mark over a navi whose trap sprang (AntiNavi, AntiRecv).
        trap_mark = effects.copy_mark_7,
        -- What an obstacle encased in ice or a bubble flickers with.
        encased = effects.explosion,
        -- A Cross or Beast Out put on.
        form_change = effects.form_change,
        -- Beast Over: its beast (Gregar's, Falzar's), the blast after it,
        -- and the bursts around the navi.
        beast_over_gregar = effects.beast_over_gregar,
        beast_over_falzar = effects.beast_over_falzar,
        beast_over_blast = effects.beast_over_blast,
        beast_over_burst = effects.beast_over_burst_45,
    },
    sparks = {
        -- What a new collision registration's hits show until its owner
        -- names another.
        plain = sparks.plain,
        -- A blocked hit's.
        guard = sparks.hit,
        -- An eruption's hits' (a volcano panel's).
        eruption = sparks.fire,
        -- A thrown obstacle's landing's.
        thrown_obstacle = sparks.charged,
        -- A navi's programs uninstalled.
        uninstall = sparks.spark_0e,
    },
    regions = {
        -- A registration's own panel, which every setup gives it.
        anchor = regions.single,
    },
    collision = {
        -- A navi's body, its body when it floats, and what it reacts to.
        navi = collision.navi,
        floating_navi = collision.floating_navi,
        navi_target = collision.navi_target,
        -- An eruption (a volcano panel's), and what it reaches.
        eruption = collision.hitbox,
        eruption_target = collision.hits_both_sides,
        -- A thrown obstacle's landing, and what it reaches.
        thrown_obstacle = collision.breaking,
        thrown_obstacle_target = collision.hits_navis,
    },
    sounds = {
        -- A panel cracks or breaks; a panel turns to poison.
        panel_crack = asset.sound("panel-crack"),
        panel_poison = asset.sound("panel-poison"),
        -- A navi hit, as its own player hears it (and the custom-HP bug's
        -- damage) and as the other does (and an object hurt without
        -- flinching); an object taking damage; a blocked hit; a counter
        -- hit; a navi deleted.
        own_hit = asset.sound("hit-6b"),
        hit = asset.sound("hit-6d"),
        damage = asset.sound("damage"),
        guard = asset.sound("hit-6e"),
        counter_hit = asset.sound("select-86"),
        deleted = asset.sound("deleted"),
        -- HP recovered; a chip's damage bonus as it is used.
        recovery = asset.sound("heal"),
        damage_bonus = asset.sound("hit-87"),
        -- The battle pauses and resumes; the custom gauge fills; the
        -- low-HP alarm.
        pause = asset.sound("pause"),
        gauge_full = asset.sound("gauge-full"),
        low_hp = asset.sound("low-hp"),
        -- A side cuts in, a trap springs; a telop shows.
        cut_in = asset.sound("dimming-sparkle"),
        telop = asset.sound("telop"),
        -- The buster starts charging, and is charged.
        buster_charge = asset.sound("buster-charge"),
        buster_charged = asset.sound("buster-charged"),
        -- A navi frozen, bubbled, its bubble popped; confusion's stars.
        freeze = asset.sound("freeze"),
        bubble = asset.sound("bubble"),
        bubble_pop = asset.sound("bubble-pop"),
        confusion = asset.sound("confusion"),
        -- A navi turns invisible, shows again (or enters), arrives
        -- mid-battle; something goes (a cancelled defensive chip,
        -- uninstalled programs, a Cross navi's special, a form taken off).
        invisible = asset.sound("invisible"),
        appear = asset.sound("appear"),
        arrive = asset.sound("point-appear"),
        fade = asset.sound("fade-8e"),
        -- A thrown obstacle lifted, and flying.
        obstacle_lift = asset.sound("point-rise"),
        obstacle_throw = asset.sound("etomahwk"),
        -- A cross merging into MegaMan; a form change starting; a Cross
        -- put on (two sounds at once); a Beast Out put on; the beasts'
        -- roars; Beast Over's rumbles and bursts.
        cross_merge = asset.sound("cross-merge"),
        form_change = asset.sound("form-change"),
        cross_change = asset.sound("cross-change"),
        cross_change_chime = asset.sound("log-in-77"),
        beast_out = asset.sound("beast-out"),
        gregar_roar = asset.sound("gregar-roar"),
        falzar_roar = asset.sound("falzar-roar"),
        beast_over_rumble = asset.sound("beast-over"),
        beast_over_burst = asset.sound("beast-over-burst"),
        -- A Cross navi starting its special's chip.
        cross_special = asset.sound("elmnt-man-2"),
        -- SELECT with too little gauge: its player hears that it can't.
        refused = asset.sound("cant-jack-in"),
    },
    music = {
        -- A link battle's, whatever its stage's.
        link_battle = asset.sound("virus-battle"),
        -- The winner's (in special battles, and otherwise) and the loser's.
        winner_special = asset.sound("winner-0"),
        winner = asset.sound("winner-1"),
        loser = asset.sound("loser"),
    },
    banners = {
        -- A round's start (the battle's number), a turn's, the final turns'.
        round_start = asset.banner("battle-number-start"),
        turn_start = asset.banner("turn-start"),
        final_turn = asset.banner("final-turn"),
        -- A draw; the damage judge's.
        draw = asset.banner("draw"),
        judge = asset.banner("held-28"),
        -- A telop on the local player's side, and on the other's.
        telop = asset.banner("telop"),
        telop_remote = asset.banner("telop-remote"),
        -- A Program Advance, and a selection that makes none.
        program_advance = asset.banner("program-advance"),
        program_advance_empty = asset.banner("program-advance-empty"),
    },
    hooks = {
        -- The NaviCust FirstBarrier (`sub_8013892`): the navi enters with
        -- the barrier its stat names, which the game's program sets to type
        -- 1 (the Barrier chip's), and its visual.
        first_barrier = function(navi: Object)
            barriers.raise(navi, barriers.barrier_10, true)
        end,
        -- What an obstacle encased in ice or a bubble becomes
        -- (`sub_801813A`'s end).
        encased = encased_bubble.replace,
    },
}
