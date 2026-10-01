--!strict
-- What the ruleset starts by role (docs/design/content-model-v2.md §7.4).
-- The trap chips' counters (AntiDmg's, AntiSwrd's, BodyGrd's) join when
-- those chips convert; until then their roles are unfilled.

local barriers = require("../lib/barriers/barriers")
local charged_shot = require("../navis/00-megaman/weapons/charged-shot/weapon")

return define.roles {
    actions = {
        -- The charged shot a navi's request starts from idle without its
        -- weapon's setup (the original's action 0x16).
        forced_charged_shot = charged_shot.action,
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
