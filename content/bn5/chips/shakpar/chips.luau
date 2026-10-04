--!strict
-- ShakPar1, ShakPar2 and ShakPar3: the shakpar series (each chip's own comment above its definition).
-- Their records are written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local recipe_spshake1 = require("../spshake/chips").spshake1
local recipe_spshake2 = require("../spshake/chips").spshake2
local recipe_spshake3 = require("../spshake/chips").spshake3
local shakpar = require("./shakpar")
local spshake = require("../spshake/spshake")

-- ShakPar1 (BN5 chip 0x14c): the Program Advance of two Voltz1s and a SpShake1 (or three SpShake1s of one code): three
-- SpShake waves along the rows from the back column, 15 ticks apart (chips/shakpar/shakpar, instant effect 22; the record's
-- parameters: speed row 2, hits that paralyze, palette 0).
local shakpar1 = define.chip {
    id = "shakpar1",
    codes = { "A", "B", "C", "*" },
    element = "elec",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 80,
    hit_param = 10,
    lockout = 20,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    -- BN5's recipes (written by tools/bn5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 48, code_run = { chip = recipe_spshake1, count = 3 } },
    },
    instant = shakpar.instant(spshake.wave(2, true, 0)),
}

-- ShakPar2 (BN5 chip 0x14d): the Program Advance of two Voltz2s and a SpShake2 (or three SpShake2s of one code): three
-- SpShake waves along the rows from the back column, 15 ticks apart (chips/shakpar/shakpar, instant effect 22; the record's
-- parameters: speed row 2, hits that paralyze, palette 6).
local shakpar2 = define.chip {
    id = "shakpar2",
    codes = { "A", "B", "C", "*" },
    element = "elec",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 110,
    hit_param = 10,
    lockout = 20,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    -- BN5's recipes (written by tools/bn5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 49, code_run = { chip = recipe_spshake2, count = 3 } },
    },
    instant = shakpar.instant(spshake.wave(2, true, 6)),
}

-- ShakPar3 (BN5 chip 0x14e): the Program Advance of two Voltz3s and a SpShake3 (or three SpShake3s of one code): three
-- SpShake waves along the rows from the back column, 15 ticks apart (chips/shakpar/shakpar, instant effect 22; the record's
-- parameters: speed row 2, hits that paralyze, palette 12).
local shakpar3 = define.chip {
    id = "shakpar3",
    codes = { "A", "B", "C", "*" },
    element = "elec",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 140,
    hit_param = 10,
    lockout = 20,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    -- BN5's recipes (written by tools/bn5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 50, code_run = { chip = recipe_spshake3, count = 3 } },
    },
    instant = shakpar.instant(spshake.wave(2, true, 12)),
}

return { shakpar1 = shakpar1, shakpar2 = shakpar2, shakpar3 = shakpar3 }
