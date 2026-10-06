--!strict
-- WaveArm1, WaveArm2 and WaveArm3: a shock wave (wave.luau) rolling from
-- the panel ahead that steers toward an enemy's row (strike.luau).

local strike = require("@self/strike")
local wave = require("@self/wave")

-- The wave (`byte_80C6B00`'s rows 0xC to 0xE, one a chip, alike): quick, it
-- steers toward an enemy's row.
local WAVE = wave.variant { anim = 1, ticks = 6, quick = true, steers = true }

local wavearm1: Chip = {
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
    slot_in_limit = 3,
    action = strike.action { id = "wavearm1/action", wave = WAVE },
}

local wavearm2: Chip = {
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
    slot_in_limit = 3,
    action = strike.action { id = "wavearm2/action", wave = WAVE },
}

local wavearm3: Chip = {
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
    slot_in_limit = 3,
    action = strike.action { id = "wavearm3/action", wave = WAVE },
}

return { wavearm1 = wavearm1, wavearm2 = wavearm2, wavearm3 = wavearm3 }
