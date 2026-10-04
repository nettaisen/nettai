--!strict
-- The barrier chips' dimming controller (the original's effect object
-- #0x2F, `sub_80E3AB8`; docs/engine/dimming-chips.md §3.1): Barrier,
-- Barr100, Barr200, BblWrap and LifeAur. The usual dimming phases
-- (lib/dimming), with an effect (`sub_80E3AFC`) that raises the chip's
-- barrier on the user with its visual (barriers.luau), then waits 61
-- ticks. The behavior is BN5's too (@exelib/barriers/controller), made here
-- with BN6's barriers.
--
-- A chip's `dimming` hook is `barrier_chips.hook(barrier)`.

local barriers = require("../barriers")
local common = require("@exelib/barriers/controller")

return common.make {
    id = "barriers/controller",
    raise = function(navi: Object, b: Barrier)
        barriers.raise(navi, b)
    end,
}
