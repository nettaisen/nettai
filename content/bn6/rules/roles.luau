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
local beast_claw = require("../navis/megaman/weapons/beast-claw/weapon")
local bodygrd_counter = require("../chips/bodygrd/counter")
local charged_shot = require("../navis/megaman/weapons/charged-shot/weapon")
local collision = require("./collision")
local dust_beast_scatter = require("../navis/megaman/forms/dustcross-beast/scatter")
local effects = require("../lib/effects")
local encased_bubble = require("../objects/encased-bubble/bubble")
local falling_rock = require("../objects/falling-rock/falling_rock")
local ground_drill = require("../navis/megaman/forms/groundcross/drill")
local lockon = require("./lockon")
local regions = require("../lib/regions")
local slash_beast_lunge = require("../navis/megaman/forms/slashcross-beast/lunge")
local slash_charge = require("../navis/megaman/forms/slashcross/charge")
local sparks = require("../lib/sparks")
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
        stun_strike = { legacy = { action = 0x49 } },
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
