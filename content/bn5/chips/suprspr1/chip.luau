--!strict
-- SuprSpr1 (BN5 chip 0x149): the Program Advance of WideSht1, WideSht1 and WideSht1 with one code: the navi raises its
-- wide shooter and sends three waves over a column of three, 14 ticks apart, that go through what they hit, the last
-- staggering harder: the shared WideSht's (@common/widesht, BN5's action 0x25 with subtype 1) three waves, at the
-- floor, of BN5's wave kind 6 (objects/wave: the record's parameters).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local wave = require("../../objects/wave/variants")
local widesht = require("../../objects/wave/wave")
local recipe_wideblde = require("../wideblde/chip")
local recipe_widesht1 = require("../widesht1/chip")
local recipe_wideswrd = require("../wideswrd/chip")

return define.chip {
    id = "bn5:suprspr1",
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
    -- BN5's recipes (written by tools/bn5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 3, sequence = { recipe_wideswrd, recipe_wideblde, recipe_widesht1 }, per_player_gauges_only = true },
        { order = 39, code_run = { chip = recipe_widesht1, count = 3 } },
    },
    action = widesht.action { wave = wave.super, waves = 3, z = 0 },
}
