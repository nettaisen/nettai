--!strict
-- PwrWave1, PwrWave2 and PwrWave3 (WaveArm's Program Advances): WaveArm's
-- strike (chips/wavearm/strike.luau) with a straight wave, and two more
-- from the navi's own column along the rows above and below.

local strike = require("../wavearm/strike")
local wave = require("../wavearm/wave")
local wavearms = require("../wavearm/chips")

local pwrwave1 = define.chip {
    id = "pwrwave1",
    name = "PwrWave1",
    description = "Send a\npwrfl wav\n3rows fwd",
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
    library = { number = 329, index = 10, sort = 0 },
    slot_in_limit = 1,
    program_advances = { { order = 31, code_run = { chip = wavearms.wavearm1, count = 3 } } },
    action = strike.action { id = "pwrwave1/action", wave = wave.variants.pwrwave, three_rows = true },
}

local pwrwave2 = define.chip {
    id = "pwrwave2",
    name = "PwrWave2",
    description = "Send a\npwrfl wav\n3rows fwd",
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
    library = { number = 330, index = 11, sort = 0 },
    slot_in_limit = 1,
    program_advances = { { order = 32, code_run = { chip = wavearms.wavearm2, count = 3 } } },
    action = strike.action { id = "pwrwave2/action", wave = wave.variants.pwrwave, three_rows = true },
}

local pwrwave3 = define.chip {
    id = "pwrwave3",
    name = "PwrWave3",
    description = "Send a\npwrfl wav\n3rows fwd",
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
    library = { number = 331, index = 12, sort = 0 },
    slot_in_limit = 1,
    program_advances = { { order = 33, code_run = { chip = wavearms.wavearm3, count = 3 } } },
    action = strike.action { id = "pwrwave3/action", wave = wave.variants.pwrwave, three_rows = true },
}

return { pwrwave1 = pwrwave1, pwrwave2 = pwrwave2, pwrwave3 = pwrwave3 }
