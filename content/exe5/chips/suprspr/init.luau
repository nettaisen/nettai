--!strict
-- SuprSpr1, SuprSpr2 and SuprSpr3: the suprspr series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local recipe_wideblde = require("./wideblde").wideblde
local recipe_widesht1 = require("./widesht").widesht1
local recipe_widesht2 = require("./widesht").widesht2
local recipe_widesht3 = require("./widesht").widesht3
local recipe_wideswrd = require("./wideswrd").wideswrd
local wave = require("./widesht/variants")
local widesht = require("./widesht/wave")

-- SuprSpr1 (EXE5 chip 0x149): the Program Advance of WideSht1, WideSht1 and WideSht1 with one code: the navi raises its
-- wide shooter and sends three waves over a column of three, 14 ticks apart, that go through what they hit, the last
-- staggering harder: the shared WideSht's (@exelib/widesht, EXE5's action 0x25 with subtype 1) three waves, at the
-- floor, of EXE5's wave kind 6 (chips/widesht: the record's parameters).
local suprspr1: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "aqua",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 100,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    -- EXE5's recipes (written by tools/exe5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 3, sequence = { recipe_wideswrd, recipe_wideblde, recipe_widesht1 }, operation_battle_only = true },
        { order = 39, code_run = { chip = recipe_widesht1, count = 3 } },
    },
    action = widesht.action { wave = wave.super, waves = 3, z = 0 },
    auto_battle = { class = "at_once" },
}

-- SuprSpr2 (EXE5 chip 0x14a): the Program Advance of WideSht2, WideSht2 and WideSht2 with one code: the navi raises its
-- wide shooter and sends three waves over a column of three, 14 ticks apart, that go through what they hit, the last
-- staggering harder: the shared WideSht's (@exelib/widesht, EXE5's action 0x25 with subtype 1) three waves, at the
-- floor, of EXE5's wave kind 7 (chips/widesht: the record's parameters).
local suprspr2: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "aqua",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 120,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    -- EXE5's recipes (written by tools/exe5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 4, sequence = { recipe_wideswrd, recipe_wideblde, recipe_widesht2 }, operation_battle_only = true },
        { order = 40, code_run = { chip = recipe_widesht2, count = 3 } },
    },
    action = widesht.action { wave = wave.super2, waves = 3, z = 0 },
    auto_battle = { class = "at_once" },
}

-- SuprSpr3 (EXE5 chip 0x14b): the Program Advance of WideSht3, WideSht3 and WideSht3 with one code: the navi raises its
-- wide shooter and sends three waves over a column of three, 14 ticks apart, that go through what they hit, the last
-- staggering harder: the shared WideSht's (@exelib/widesht, EXE5's action 0x25 with subtype 1) three waves, at the
-- floor, of EXE5's wave kind 8 (chips/widesht: the record's parameters).
local suprspr3: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "aqua",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 140,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    -- EXE5's recipes (written by tools/exe5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 5, sequence = { recipe_wideswrd, recipe_wideblde, recipe_widesht3 }, operation_battle_only = true },
        { order = 41, code_run = { chip = recipe_widesht3, count = 3 } },
    },
    action = widesht.action { wave = wave.super3, waves = 3, z = 0 },
    auto_battle = { class = "at_once" },
}

return { suprspr1 = suprspr1, suprspr2 = suprspr2, suprspr3 = suprspr3 }
