--!strict
-- Barrier, Barr100 and Barr200: the barrier series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local barriers = require("../lib/barriers")

-- Barrier (EXE5 chip 0x087): a barrier of 10 HP on the user: EXE6's barrier chips (the dimming handler's subtype 14)
-- with EXE5's barriers (lib/barriers).
local barrier: Chip = {
    codes = { "B", "L", "S", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 7,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 1,
    megaman = "light",
    dimming = barriers.controller.hook(barriers.barrier_10),
}

-- Barr100 (EXE5 chip 0x088): a barrier of 100 HP on the user: EXE6's barrier chips (the dimming handler's subtype 14)
-- with EXE5's barriers (lib/barriers).
local barr100: Chip = {
    codes = { "P" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 27,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 1,
    megaman = "light",
    dimming = barriers.controller.hook(barriers.barrier_100),
}

-- Barr200 (EXE5 chip 0x089): a barrier of 200 HP on the user: EXE6's barrier chips (the dimming handler's subtype 14)
-- with EXE5's barriers (lib/barriers).
local barr200: Chip = {
    codes = { "W" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 47,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 1,
    megaman = "light",
    dimming = barriers.controller.hook(barriers.barrier_200),
}

return { barrier = barrier, barr100 = barr100, barr200 = barr200 }
