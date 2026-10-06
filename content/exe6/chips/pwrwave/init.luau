--!strict
-- PwrWave1, PwrWave2 and PwrWave3 (WaveArm's Program Advances): WaveArm's
-- strike (chips/wavearm/strike.luau) with a straight wave, and two more
-- from the navi's own column along the rows above and below.

local strike = require("./wavearm/strike")
local wave = require("./wavearm/wave")
local wavearm_chips = require("./wavearm")

local pwrwave1: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 400,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    program_advances = { { order = 31, code_run = { chip = wavearm_chips.wavearm1, count = 3 } } },
    action = strike.action { id = "pwrwave1/action", wave = wave.variants.pwrwave, three_rows = true },
}

local pwrwave2: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 500,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    program_advances = { { order = 32, code_run = { chip = wavearm_chips.wavearm2, count = 3 } } },
    action = strike.action { id = "pwrwave2/action", wave = wave.variants.pwrwave, three_rows = true },
}

local pwrwave3: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 600,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    program_advances = { { order = 33, code_run = { chip = wavearm_chips.wavearm3, count = 3 } } },
    action = strike.action { id = "pwrwave3/action", wave = wave.variants.pwrwave, three_rows = true },
}

return { pwrwave1 = pwrwave1, pwrwave2 = pwrwave2, pwrwave3 = pwrwave3 }
