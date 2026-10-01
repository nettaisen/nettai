--!strict
-- Registration by number (docs/design/content-model-v2.md §12,
-- "Transitional"): a record that names the original's action 0x26 runs
-- this module, which runs the chip's own flick (chips/airhocky,
-- chips/pithocky). The records that reach it are AirHocky's (PitHocky's
-- recipe and the Cross special name it by number) and PitHocky's (a
-- Program Advance); they differ in their puck (the record's first
-- parameter: 3 is PitHocky's). It goes when they are definitions (step 5,
-- phase C).

local airhocky = require("../airhocky/chip")
local flick = require("../airhocky/flick")
local pithocky = require("../pithocky/chip")

local by_number = {
    state = flick.STATE,
}

-- PitHocky's puck.
local PIT_PUCK = 3

function by_number.update(me: Object, s: any)
    local a = if me:attack_param(1) == PIT_PUCK then pithocky.action else airhocky.action
    a.update(me, s)
end

return by_number
