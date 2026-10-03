--!strict
-- TimeBom1, TimeBom2, TimeBom3 and TimeBom+: the timebom series (each chip's own comment above its definition).
-- Their records are written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local timebom = require("./timebom")

-- TimeBom1 (BN5 chip 0x04b): a bomb on the enemy's frontmost panel of a random row that counts 3, 2, 1 and blows
-- up the whole enemy area unless it is broken first (chips/timebom/timebom: BN6's TimeBom with BN5's placement).
local timebom1 = define.chip {
    id = "bn5:timebom1",
    codes = { "H", "N", "T" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 0,
    mb = 32,
    damage = 100,
    hit_param = 148,
    lockout = 30,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    dimming = timebom.controller.hook(timebom.variants.bomb),
}

-- TimeBom2 (BN5 chip 0x04c): a bomb on the enemy's frontmost panel of a random row that counts 3, 2, 1 and blows
-- up the whole enemy area unless it is broken first (chips/timebom/timebom: BN6's TimeBom with BN5's placement).
local timebom2 = define.chip {
    id = "bn5:timebom2",
    codes = { "D", "H", "Z" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 1,
    mb = 54,
    damage = 140,
    hit_param = 148,
    lockout = 30,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    dimming = timebom.controller.hook(timebom.variants.bomb),
}

-- TimeBom3 (BN5 chip 0x04d): a bomb on the enemy's frontmost panel of a random row that counts 3, 2, 1 and blows
-- up the whole enemy area unless it is broken first (chips/timebom/timebom: BN6's TimeBom with BN5's placement).
local timebom3 = define.chip {
    id = "bn5:timebom3",
    codes = { "F", "H", "S" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 2,
    mb = 76,
    damage = 200,
    hit_param = 148,
    lockout = 30,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    dimming = timebom.controller.hook(timebom.variants.bomb),
}

-- TimeBom+ (BN5 chip 0x15b): the Program Advance of TimeBom1, TimeBom2 and TimeBom3 in that order: TimeBom1-3's bomb
-- with 300 HP, its own look and a harder shake (chips/timebom/timebom: BN6's TimeBom with BN5's placement).
local timebom_chip = define.chip {
    id = "bn5:timebom",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 600,
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    slot_in_limit = 1,
    -- BN5's recipes are in the order of its full table (0x08027FC8; the netbattles' 0x0802801C is that table but
    -- for its first 21, which only a battle with each side keeping its own gauge tries): its 24th, the
    -- netbattles' third.
    program_advances = { { order = 23, sequence = { timebom1, timebom2, timebom3 } } },
    dimming = timebom.controller.hook(timebom.variants.big),
}

return { timebom1 = timebom1, timebom2 = timebom2, timebom3 = timebom3, timebom = timebom_chip }
