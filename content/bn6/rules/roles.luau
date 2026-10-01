--!strict
-- What the ruleset starts and spawns by role (docs/design/content-model-v2.md
-- §7.4). A role whose target is still a v1 registration names it through
-- the transitional `legacy` marker (an action by its number, a kind by its
-- key), counted by the ratchet; a family that converts the target names its
-- definition here instead. The trap chips' counters (AntiDmg's, AntiSwrd's,
-- BodyGrd's) join when those chips convert; until then their roles are
-- unfilled.

local barriers = require("../lib/barriers/barriers")
local charged_shot = require("../navis/00-megaman/weapons/charged-shot/weapon")
local supports = require("../lib/supports/controller")

return define.roles {
    actions = {
        -- The charged shot a navi's request starts from idle without its
        -- weapon's setup (the original's action 0x16).
        forced_charged_shot = charged_shot.action,
        -- The strike the navi's request 0x80000 starts from idle (the
        -- machine swords' strike by the attack's variant).
        stun_strike = { legacy = { action = 0x49 } },
        -- What a Cross navi protected at the battle's end runs.
        cross_protect = { legacy = { action = 0x4D } },
        -- The turn L or R starts where turning is enabled.
        turn = { legacy = { action = 0x3B } },
        -- A Cross navi's knock-out.
        cross_death = { legacy = { action = 0x4C } },
        -- A Cross navi's volley.
        volley = { legacy = { action = 0x30 } },
        -- SlashCross's charged slash, which the Beast rush recognizes.
        charged_sword = { legacy = { action = 0x41 } },
        -- The Beast forms' charged claw.
        beast_claw = { legacy = { action = 0x52 } },
        -- DustCross Beast's scatter, during which the navi isn't grounded.
        dust_beast_scatter = { legacy = { action = 0x50 } },
    },
    kinds = {
        -- The rock a stage's actor list places (entry type 8).
        rock = { legacy = { kind = "rock" } },
        -- What an obstacle absorbed flies off as.
        absorbed_obstacle = { legacy = { kind = "absorbed-obstacle" } },
        -- GroundCross's charged chips' falling rocks.
        falling_rock = { legacy = { kind = "falling-rock" } },
        -- The NaviCust supports' controller (Rush's, Beat's and Tango's).
        support = supports.kind,
    },
    hooks = {
        -- The NaviCust FirstBarrier (`sub_8013892`): the navi enters with
        -- the barrier its stat names, which the game's program sets to type
        -- 1 (the Barrier chip's), and its visual.
        first_barrier = function(navi: Object)
            barriers.raise(navi, barriers.barrier_10, true)
        end,
    },
}
