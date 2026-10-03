--!strict
-- CacDanc2 (BN5 chip 0x150): the Program Advance of two WoodNos2s and a CactBal2 (or three CactBal2s of one code): an
-- unseen dancer drops twelve cactuses on the other side's navis (objects/cacdanc, instant effect 23; the cactuses'
-- palette the record's first parameter, 1, times 2).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local cacdanc = require("../../objects/cacdanc/cacdanc")
local recipe_cactbal2 = require("../cactbal2/chip")
local recipe_woodnos2 = require("../woodnos2/chip")

return define.chip {
    id = "bn5:cacdanc2",
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
    -- BN5's recipes (written by tools/bn5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 16, sequence = { recipe_woodnos2, recipe_woodnos2, recipe_cactbal2 }, per_player_gauges_only = true },
        { order = 52, code_run = { chip = recipe_cactbal2, count = 3 } },
    },
    instant = cacdanc.instant(1),
}
