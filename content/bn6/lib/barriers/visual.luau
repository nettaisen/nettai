--!strict
-- A barrier's visual (the original's effect object #7, `sub_80E0AD4`;
-- docs/engine/dimming-chips.md §3.3): it sits on its navi at the look's
-- attach point and follows the navi's barrier (the collision data's):
-- up, it shows; worn down, it goes down (and comes back up when the
-- barrier grows back); popped by wind, it is blown away; with the barrier
-- gone, it goes. The navi links it (`barrier_visual`, AIData+0x60), and
-- hides and shows it with itself (its `shown`). The behavior is BN5's too
-- (@common/barriers/visual), made here as BN6's: blown away when popped.

local common = require("@common/barriers/visual")

local visual = {}

local made = common.make { id = "bn6:barrier-visual", blows = true }
visual.kind = made.kind

-- `sub_80E0D98`: `b`'s visual on `navi`, which links it; it runs while
-- paused too.
visual.spawn = made.spawn

-- `sub_80E0DC0`: the visual is told to end.
visual.finish = common.finish

-- `sub_801A7CC`: `b` on `navi`'s collision data.
visual.raise_barrier = common.raise_barrier

return visual
