--!strict
-- AirHocky: a puck flicked from the panel ahead, slanting down the field,
-- that bounces around for 10 steps (flick.luau).

local flick = require("@self/flick")
local puck = require("@self/puck")
local lockon = require("../rules/lockon")

-- The puck (`byte_80C9818`'s row 2): 10 steps.
local PUCK = puck.variant { steps = 0x0A, own_side = 1, ticks = 6 }

local chip: Chip = {
    codes = { "L", "M", "N" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 2,
    mb = 19,
    damage = 60,
    hit_param = 5,
    lockout = 50,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.thunder },
    slot_in_limit = 3,
    action = flick.action { id = "airhocky/action", puck = PUCK, down = true },
}

return { airhocky = chip }
