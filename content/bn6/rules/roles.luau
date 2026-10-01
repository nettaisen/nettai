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

return define.roles {
    actions = {
        -- The charged shot a navi's request starts from idle without its
        -- weapon's setup (the original's action 0x16).
        forced_charged_shot = charged_shot.action,
    },
    kinds = {
        -- The rock a stage's actor list places (entry type 8).
        rock = { legacy = { kind = "rock" } },
        -- What an obstacle absorbed flies off as.
        absorbed_obstacle = { legacy = { kind = "absorbed-obstacle" } },
        -- GroundCross's charged chips' falling rocks.
        falling_rock = { legacy = { kind = "falling-rock" } },
        -- The NaviCust supports' controller (effect object #0x79).
        support = { legacy = { kind = "support" } },
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
