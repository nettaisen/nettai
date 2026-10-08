--!strict
-- CircGun1 to CircGun3: the circgun series, the dimming chips' variant 0x33: a gun sight circles the enemy's area
-- until A or 360 ticks, then leaves four shots on the panels it passes (./circgun: EXE4's CircGun on @exelib/circgun);
-- each chip its sight's period (its first parameter: 6, 4, 2), four shots in palette 0, thrown hits (each chip's own
-- comment above its definition).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local circgun = require("@self/circgun")
local collision = require("../rules/collision")

-- CircGun1 (EXE4 chip 0x042): a step every 7 ticks.
local circgun1: Chip = {
    codes = { "H", "T", "Z" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 28,
    damage = 80,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = circgun.hook(circgun.variant { period = 6, shots = 4, palette = 0, hit = collision.thrown }),
}

-- CircGun2 (EXE4 chip 0x043): a step every 5 ticks.
local circgun2: Chip = {
    codes = { "D", "G", "T" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 40,
    damage = 100,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = circgun.hook(circgun.variant { period = 4, shots = 4, palette = 0, hit = collision.thrown }),
}

-- CircGun3 (EXE4 chip 0x044): a step every 3 ticks.
local circgun3: Chip = {
    codes = { "J", "M", "R" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 52,
    damage = 120,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = circgun.hook(circgun.variant { period = 2, shots = 4, palette = 0, hit = collision.thrown }),
}

return { circgun1 = circgun1, circgun2 = circgun2, circgun3 = circgun3 }
