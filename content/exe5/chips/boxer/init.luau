--!strict
-- Boxer1, Boxer2 and Boxer3: the boxer series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local boxer = require("@self/boxer")
local recipe_firehit1 = require("./firehit").firehit1
local recipe_firehit2 = require("./firehit").firehit2
local recipe_firehit3 = require("./firehit").firehit3
local recipe_hotbody1 = require("./hotbody").hotbody1
local recipe_hotbody2 = require("./hotbody").hotbody2
local recipe_hotbody3 = require("./hotbody").hotbody3

-- Boxer1 (EXE5 chip 0x146): the Program Advance of two HotBody1s and a FireHit1 (or three FireHit1s of one code): an
-- unseen boxer punches FireHit's fists down the rows, nine in all (chips/boxer/boxer, instant effect 21; the fists' hit
-- modifier 1, the record's +0x11).
local boxer1: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "fire",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 100,
    hit_param = 10,
    lockout = 20,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    -- EXE5's recipes (written by tools/exe5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 9, sequence = { recipe_hotbody1, recipe_hotbody1, recipe_firehit1 }, operation_battle_only = true },
        { order = 45, code_run = { chip = recipe_firehit1, count = 3 } },
    },
    instant = boxer.instant(1),
}

-- Boxer2 (EXE5 chip 0x147): the Program Advance of two HotBody2s and a FireHit2 (or three FireHit2s of one code): an
-- unseen boxer punches FireHit's fists down the rows, nine in all (chips/boxer/boxer, instant effect 21; the fists' hit
-- modifier 1, the record's +0x11).
local boxer2: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "fire",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 150,
    hit_param = 10,
    lockout = 20,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    -- EXE5's recipes (written by tools/exe5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 10, sequence = { recipe_hotbody2, recipe_hotbody2, recipe_firehit2 }, operation_battle_only = true },
        { order = 46, code_run = { chip = recipe_firehit2, count = 3 } },
    },
    instant = boxer.instant(1),
}

-- Boxer3 (EXE5 chip 0x148): the Program Advance of two HotBody3s and a FireHit3 (or three FireHit3s of one code): an
-- unseen boxer punches FireHit's fists down the rows, nine in all (chips/boxer/boxer, instant effect 21; the fists' hit
-- modifier 1, the record's +0x11).
local boxer3: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "fire",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 200,
    hit_param = 10,
    lockout = 20,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    -- EXE5's recipes (written by tools/exe5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 11, sequence = { recipe_hotbody3, recipe_hotbody3, recipe_firehit3 }, operation_battle_only = true },
        { order = 47, code_run = { chip = recipe_firehit3, count = 3 } },
    },
    instant = boxer.instant(1),
}

return { boxer1 = boxer1, boxer2 = boxer2, boxer3 = boxer3 }
