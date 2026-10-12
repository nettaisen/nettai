--!strict
-- PitHoky1, PitHoky2 and PitHoky3 (EXE4 chips 0x152 to 0x154; their records at 0x080197EC + n * 0x2C): the Program
-- Advances of three AirHoc1s, AirHoc2s or AirHoc3s of consecutive codes (the recipe table, the pointer list at
-- 0x080212B0). AirHoc's flick (@exelib/airhocky/flick: EXE4's action 0x27) of EXE4's puck (./airhoc/puck) in its row 9
-- (0x080D2768: its Param1; its Param2 1, down): 18 steps, leaving a trail (0x080D2ACC).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local airhocs = require("./airhoc")
local flick = require("@exelib/airhocky/flick")
local puck = require("./airhoc/puck")

-- The navi's animation as it flicks (0x080ECED4).
local ANIM = 0x10

-- Row 9: 18 steps, 6 ticks each, a trail every fourth tick (an afterimage of the puck for 8 ticks, tinted 0x8318).
local PUCK = puck.variant { steps = 0x12, own_side = 1, ticks = 6, trail = 0x8318 }

local function action(id: string): Action
    return flick.action { id = id, kind = puck.kind, puck = PUCK, down = true, anim = ANIM }
end

local pithoky1: Chip = {
    action = action("pithoky1/action"),
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 100,
    hit_param = 50,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = { { order = 29, code_run = { chip = airhocs.airhoc1, count = 3 } } },
}

local pithoky2: Chip = {
    action = action("pithoky2/action"),
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 150,
    hit_param = 50,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = { { order = 30, code_run = { chip = airhocs.airhoc2, count = 3 } } },
}

local pithoky3: Chip = {
    action = action("pithoky3/action"),
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 200,
    hit_param = 50,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = { { order = 31, code_run = { chip = airhocs.airhoc3, count = 3 } } },
}

return { pithoky1 = pithoky1, pithoky2 = pithoky2, pithoky3 = pithoky3 }
