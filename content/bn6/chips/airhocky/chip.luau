--!strict
-- AirHocky: a puck flicked from the panel ahead, slanting down the field,
-- that bounces around for 10 steps (flick.luau).
--
-- Its record stays the pack's for now (PitHocky's Program Advance recipe
-- names it by number, as does the Cross special): this is its action,
-- which registration by number runs (chips/032-airhocky,
-- docs/design/content-model-v2.md §12).

local flick = require("./flick")
local puck = require("./puck")

return {
    action = flick.action { id = "airhocky/action", puck = puck.variants[2], down = true },
}
