--!strict
-- AirHoc1, AirHoc2 and AirHoc3 (EXE4 chips 0x03F to 0x041; their records at 0x080197EC + n * 0x2C): a puck flicked from
-- the panel ahead, slanting down the field, that bounces around for 6, 10 or 14 steps: the shared flick
-- (@exelib/airhocky/flick: EXE4's action 0x27, EXE5's but for the navi's animation, 0x10) and EXE4's puck (./puck), each
-- chip's row of 0x080D2768 (its Param1: 6 to 8; its Param2 1, down).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local flick = require("@exelib/airhocky/flick")
local puck = require("@self/puck")

-- The navi's animation as it flicks (0x080ECED4).
local ANIM = 0x10

local airhoc1: Chip = {
    action = flick.action {
        id = "airhoc1/action",
        kind = puck.kind,
        -- Row 6: 6 steps.
        puck = puck.variant { steps = 6, own_side = 1, ticks = 6 },
        down = true,
        anim = ANIM,
    },
    codes = { "D", "E", "F", "*" },
    element = "null",
    family = "metal",
    class = "standard",
    rarity = 0,
    mb = 20,
    damage = 50,
    hit_param = 20,
    lockout = 15,
    flags = { "has_damage", "standard_library", "library" },
}

local airhoc2: Chip = {
    action = flick.action {
        id = "airhoc2/action",
        kind = puck.kind,
        -- Row 7: 10 steps.
        puck = puck.variant { steps = 0x0A, own_side = 1, ticks = 6 },
        down = true,
        anim = ANIM,
    },
    codes = { "I", "J", "K" },
    element = "null",
    family = "metal",
    class = "standard",
    rarity = 1,
    mb = 30,
    damage = 60,
    hit_param = 20,
    lockout = 40,
    flags = { "has_damage", "standard_library", "library" },
}

local airhoc3: Chip = {
    action = flick.action {
        id = "airhoc3/action",
        kind = puck.kind,
        -- Row 8: 14 steps.
        puck = puck.variant { steps = 0x0E, own_side = 1, ticks = 6 },
        down = true,
        anim = ANIM,
    },
    codes = { "U", "V", "W" },
    element = "null",
    family = "metal",
    class = "standard",
    rarity = 2,
    mb = 40,
    damage = 70,
    hit_param = 20,
    lockout = 50,
    flags = { "has_damage", "standard_library", "library" },
}

return { airhoc1 = airhoc1, airhoc2 = airhoc2, airhoc3 = airhoc3 }
