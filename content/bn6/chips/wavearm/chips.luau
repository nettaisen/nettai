--!strict
-- WaveArm1, WaveArm2 and WaveArm3: a shock wave (wave.luau) rolling from
-- the panel ahead that steers toward an enemy's row (strike.luau).

local strike = require("./strike")
local wave = require("./wave")

local wavearm1 = define.chip {
    id = "wavearm1",
    name = "WaveArm1",
    description = "Fllw enmy\nand fire\ntrap wave",
    codes = { "E", "F", "G" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 15,
    damage = 80,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 94, index = 94, sort = 503 },
    slot_in_limit = 3,
    action = strike.action { id = "wavearm1/action", wave = wave.variants.wavearm1 },
}

local wavearm2 = define.chip {
    id = "wavearm2",
    name = "WaveArm2",
    description = "Fllw enmy\nand fire\ntrap wave",
    codes = { "L", "M", "N" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 22,
    damage = 120,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 95, index = 95, sort = 504 },
    slot_in_limit = 3,
    action = strike.action { id = "wavearm2/action", wave = wave.variants.wavearm2 },
}

local wavearm3 = define.chip {
    id = "wavearm3",
    name = "WaveArm3",
    description = "Fllw enmy\nand fire\ntrap wave",
    codes = { "R", "S", "T" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 29,
    damage = 160,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 96, index = 96, sort = 505 },
    slot_in_limit = 3,
    action = strike.action { id = "wavearm3/action", wave = wave.variants.wavearm3 },
}

return { wavearm1 = wavearm1, wavearm2 = wavearm2, wavearm3 = wavearm3 }
