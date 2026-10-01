--!strict
-- What the ruleset needs from content by role (docs/design/
-- content-model-v2.md §7.4). One definition; groups fill in as their
-- owners convert.

local barriers = require("../lib/barriers/barriers")

return define.roles {
    hooks = {
        -- The NaviCust FirstBarrier (`sub_8013892`): the navi enters with
        -- the barrier its stat names, which the game's program sets to type
        -- 1 (the Barrier chip's), and its visual.
        first_barrier = function(navi: Object)
            barriers.raise(navi, barriers.barrier_10, true)
        end,
    },
}
