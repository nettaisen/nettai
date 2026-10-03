--!strict
-- Boxer2 (BN5 chip 0x147): the Program Advance of two HotBody2s and a FireHit2 (or three FireHit2s of one code): an
-- unseen boxer punches FireHit's fists down the rows, nine in all (objects/boxer, instant effect 21; the fists' hit
-- modifier 1, the record's +0x11).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local boxer = require("../../objects/boxer/boxer")
local recipe_firehit2 = require("../firehit2/chip")
local recipe_hotbody2 = require("../hotbody2/chip")

return define.chip {
    id = "bn5:boxer2",
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
    -- BN5's recipes (written by tools/bn5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 10, sequence = { recipe_hotbody2, recipe_hotbody2, recipe_firehit2 }, per_player_gauges_only = true },
        { order = 46, code_run = { chip = recipe_firehit2, count = 3 } },
    },
    instant = boxer.instant(1),
}
