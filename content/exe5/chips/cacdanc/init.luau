--!strict
-- CacDanc1, CacDanc2 and CacDanc3: the cacdanc series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local cacdanc = require("@self/cacdanc")
local recipe_cactbal1 = require("./cactbal").cactbal1
local recipe_cactbal2 = require("./cactbal").cactbal2
local recipe_cactbal3 = require("./cactbal").cactbal3
local recipe_woodnos1 = require("./woodnos").woodnos1
local recipe_woodnos2 = require("./woodnos").woodnos2
local recipe_woodnos3 = require("./woodnos").woodnos3

-- CacDanc1 (EXE5 chip 0x14f): the Program Advance of two WoodNos1s and a CactBal1 (or three CactBal1s of one code): an
-- unseen dancer drops twelve cactuses on the other side's navis (chips/cacdanc/cacdanc, instant effect 23; the cactuses'
-- palette the record's first parameter, 0, times 2).
local cacdanc1: Chip = {
    codes = {},
    element = "wood",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 30,
    hit_param = 5,
    lockout = 20,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    -- EXE5's recipes (written by tools/exe5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 15, sequence = { recipe_woodnos1, recipe_woodnos1, recipe_cactbal1 }, operation_battle_only = true },
        { order = 51, code_run = { chip = recipe_cactbal1, count = 3 } },
    },
    instant = cacdanc.instant(0),
    auto_battle = { class = "at_once" },
}

-- CacDanc2 (EXE5 chip 0x150): the Program Advance of two WoodNos2s and a CactBal2 (or three CactBal2s of one code): an
-- unseen dancer drops twelve cactuses on the other side's navis (chips/cacdanc/cacdanc, instant effect 23; the cactuses'
-- palette the record's first parameter, 1, times 2).
local cacdanc2: Chip = {
    codes = {},
    element = "wood",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 40,
    hit_param = 5,
    lockout = 20,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    -- EXE5's recipes (written by tools/exe5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 16, sequence = { recipe_woodnos2, recipe_woodnos2, recipe_cactbal2 }, operation_battle_only = true },
        { order = 52, code_run = { chip = recipe_cactbal2, count = 3 } },
    },
    instant = cacdanc.instant(1),
    auto_battle = { class = "at_once" },
}

-- CacDanc3 (EXE5 chip 0x151): the Program Advance of two WoodNos3s and a CactBal3 (or three CactBal3s of one code): an
-- unseen dancer drops twelve cactuses on the other side's navis (chips/cacdanc/cacdanc, instant effect 23; the cactuses'
-- palette the record's first parameter, 2, times 2).
local cacdanc3: Chip = {
    codes = {},
    element = "wood",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 50,
    hit_param = 5,
    lockout = 20,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    -- EXE5's recipes (written by tools/exe5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 17, sequence = { recipe_woodnos3, recipe_woodnos3, recipe_cactbal3 }, operation_battle_only = true },
        { order = 53, code_run = { chip = recipe_cactbal3, count = 3 } },
    },
    instant = cacdanc.instant(2),
    auto_battle = { class = "at_once" },
}

return { cacdanc1 = cacdanc1, cacdanc2 = cacdanc2, cacdanc3 = cacdanc3 }
