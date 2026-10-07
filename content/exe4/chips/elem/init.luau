--!strict
-- ElemFlar, ElemIce, ElemLeaf, ElemSand and ElemDark (EXE4 chips 0x019, 0x01A, 0x07E, 0x080 and 0x0D5; their records at
-- 0x080197EC + n * 0x2C): action 0x60 (./elem), its variants 0 to 4: three flames along the row for 60 ticks each,
-- five and paralyzing on the variant's panel (their parameters: 3 flames, 60 ticks; ElemDark's flames row 0x25
-- attacks, its flag 8).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local elem = require("@self/elem")

local COUNT, TICKS = 3, 60

local elemflar: Chip = {
    action = elem.action { id = "elemflar/action", variant = elem.variants.flare, count = COUNT, ticks = TICKS, flags = 0 },
    codes = { "K", "N", "P" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 2,
    mb = 24,
    damage = 100,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local elemice: Chip = {
    action = elem.action { id = "elemice/action", variant = elem.variants.ice, count = COUNT, ticks = TICKS, flags = 0 },
    codes = { "H", "Q", "V" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 2,
    mb = 24,
    damage = 100,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local elemleaf: Chip = {
    action = elem.action { id = "elemleaf/action", variant = elem.variants.leaf, count = COUNT, ticks = TICKS, flags = 0 },
    codes = { "C", "D", "I" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 2,
    mb = 24,
    damage = 80,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local elemsand: Chip = {
    action = elem.action { id = "elemsand/action", variant = elem.variants.sand, count = COUNT, ticks = TICKS, flags = 0 },
    codes = { "A", "L", "S", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 24,
    damage = 120,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local elemdark: Chip = {
    action = elem.action { id = "elemdark/action", variant = elem.variants.dark, count = COUNT, ticks = TICKS, flags = 8 },
    codes = { "E" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 38,
    damage = 220,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "library" },
}

return { elemflar = elemflar, elemice = elemice, elemleaf = elemleaf, elemsand = elemsand, elemdark = elemdark }
