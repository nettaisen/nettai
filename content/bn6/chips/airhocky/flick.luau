--!strict
-- AirHocky's and PitHocky's flick (the original's action 0x26,
-- `sub_80ECCB0`): the shared flick (@common/airhocky/flick) with BN6's
-- puck (puck.luau). The navi flicks a puck from the panel ahead on the
-- third tick, slanting down the field or up; it bounces around the field,
-- a panel per 6 ticks. 31 ticks in all.

local common = require("@common/airhocky/flick")
local puck = require("./puck")

local flick = {}

-- One state for both flicks.
flick.STATE = common.STATE

-- A chip's flick: its puck's variant and whether it goes down the field;
-- `id`, the action's key (its compat key: both chips are records).
function flick.action(spec: { id: string, puck: HockeyPuckVariant, down: boolean }): Action
    return common.action { id = spec.id, kind = puck.kind, puck = spec.puck, down = spec.down }
end

return flick
