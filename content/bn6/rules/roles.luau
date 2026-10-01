--!strict
-- What the ruleset starts and spawns by role (docs/design/content-model-v2.md
-- §7.4): each role names the definition that fills it.

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
local dust_beast_scatter = require("../navis/megaman/forms/dustcross-beast/scatter")
local encased_bubble = require("../objects/encased-bubble/bubble")
local invalid = require("../chips/invalid/chip")
local megabstr = require("../chips/megabstr/chip")
local rush = require("../chips/rush/chip")
local tango = require("../chips/tango/chip")
local turn = require("../navis/megaman/turn")
local widesht = require("../chips/widesht/action")
local falling_rock = require("../objects/falling-rock/falling_rock")
local ground_drill = require("../navis/megaman/forms/groundcross/drill")
local lockon = require("./lockon")
local slash_beast_lunge = require("../navis/megaman/forms/slashcross-beast/lunge")
local slash_charge = require("../navis/megaman/forms/slashcross/charge")
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
        -- The strike the navi's request 0x80000 starts from idle (a
        -- machine sword's, by the attack's chip).
        stun_strike = stun_strike.action,
        -- What a Cross navi protected at the battle's end runs
        -- (GroundCross's drill).
        cross_protect = ground_drill.action,
        -- The turn L or R starts where turning is enabled.
        turn = turn.action,
        -- A Cross navi's knock-out (SlashCross Beast's lunge).
        cross_death = slash_beast_lunge.action,
        -- What the navi's volley request starts: the original's action
        -- 0x30, which is WideSht's. (Nothing raises the request.)
        volley = widesht.actions.widesht,
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
