--!strict
-- Barrier, Barr100, Barr200, LifeAura and BlakBarr: the barrier series, the dimming chips' variant 0x15: the barrier
-- chips' controller (lib/barriers) raising each chip's barrier on its user (each chip's own comment above its
-- definition).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local barriers = require("../lib/barriers")

-- Barrier (EXE4 chip 0x087): a barrier of 10 HP (type 1).
local barrier: Chip = {
    codes = { "A", "D", "Z", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 7,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = barriers.controller.hook(barriers.barrier_10),
}

-- Barr100 (EXE4 chip 0x088): a barrier of 100 HP (type 2).
local barr100: Chip = {
    codes = { "P", "Q", "W" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 27,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = barriers.controller.hook(barriers.barrier_100),
}

-- Barr200 (EXE4 chip 0x089): a barrier of 200 HP (type 3).
local barr200: Chip = {
    codes = { "I", "O", "U" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 47,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = barriers.controller.hook(barriers.barrier_200),
}

-- LifeAura (EXE4 chip 0x0C9): an aura that stops every hit of less than 200 for 3000 ticks (type 6).
local lifeaura: Chip = {
    codes = { "D" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 4,
    mb = 70,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library" },
    dimming = barriers.controller.hook(barriers.life_aura),
}

-- BlakBarr (EXE4 chip 0x134, Red Sun's): a barrier of 150 HP, back whole 180 ticks after it is worn down (type 4).
local blakbarr: Chip = {
    codes = { "A" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 87,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library" },
    dimming = barriers.controller.hook(barriers.black_barrier),
}

return { barrier = barrier, barr100 = barr100, barr200 = barr200, lifeaura = lifeaura, blakbarr = blakbarr }
