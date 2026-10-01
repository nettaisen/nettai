--!strict
-- PitHocky (AirHocky's Program Advance): AirHocky's flick
-- (chips/airhocky/flick.luau) of a puck that bounces for 18 steps,
-- leaving a trail.
--
-- Its record stays the pack's for now (a Program Advance): this is its
-- action, which registration by number runs (chips/032-airhocky,
-- docs/design/content-model-v2.md §12).

local flick = require("../airhocky/flick")
local puck = require("../airhocky/puck")

return {
    action = flick.action { id = "pithocky/action", puck = puck.variants[3], down = true },
}
