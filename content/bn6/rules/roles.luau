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
local beast_claw = require("../navis/00-megaman/weapons/beast-claw/weapon")
local bodygrd_counter = require("../chips/bodygrd/counter")
local boulder = require("../objects/boulder/boulder")
local charged_shot = require("../navis/00-megaman/weapons/charged-shot/weapon")
local dust_beast_scatter = require("../navis/00-megaman/forms/dustcross-beast/scatter")
local encased_bubble = require("../objects/encased-bubble/bubble")
local falling_rock = require("../objects/falling-rock/falling_rock")
local ground_drill = require("../navis/00-megaman/forms/groundcross/drill")
local guardian_statue = require("../chips/guardian/statue")
local rock = require("../objects/rock/rock")
local slash_beast_lunge = require("../navis/00-megaman/forms/slashcross-beast/lunge")
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
        charged_sword = { legacy = { action = 0x41 } },
        -- The Beast forms' charged claw.
        beast_claw = beast_claw.action,
        -- DustCross Beast's scatter, during which the navi isn't grounded.
        dust_beast_scatter = dust_beast_scatter.action,
    },
    kinds = {
        -- What a stage's actor list places, by its `place`: the rock (entry
        -- type 8), the boulder (3) and the Guardian statue (9).
        rock = rock.kind,
        boulder = boulder.kind,
        statue = guardian_statue.kind,
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
