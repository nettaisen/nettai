--!strict
-- PitHocky (AirHocky's Program Advance): AirHocky's flick
-- (chips/airhocky/flick.luau) of a puck that bounces for 18 steps,
-- leaving a trail.

local flick = require("./airhocky/flick")
local puck = require("./airhocky/puck")
local airhocky = require("./airhocky").airhocky

-- The puck (`byte_80C9818`'s row 3): 18 steps, with a trail.
local PUCK = puck.variant { steps = 0x12, own_side = 1, ticks = 6, trail = 0x8318 }

local chip: Chip = {
    codes = {},
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 100,
    hit_param = 5,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    program_advances = { { order = 26, code_run = { chip = airhocky, count = 3 } } },
    action = flick.action { id = "pithocky/action", puck = PUCK, down = true },
}

return { pithocky = chip }
