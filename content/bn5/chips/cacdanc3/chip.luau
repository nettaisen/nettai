--!strict
-- CacDanc3 (BN5 chip 0x151): the Program Advance of two WoodNos3s and a CactBal3 (or three CactBal3s of one code): an
-- unseen dancer drops twelve cactuses on the other side's navis (objects/cacdanc, instant effect 23; the cactuses'
-- palette the record's first parameter, 2, times 2).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local cacdanc = require("../../objects/cacdanc/cacdanc")
local recipe_cactbal3 = require("../cactbal3/chip")
local recipe_woodnos3 = require("../woodnos3/chip")

return define.chip {
    id = "bn5:cacdanc3",
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
    -- BN5's recipes (written by tools/bn5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 17, sequence = { recipe_woodnos3, recipe_woodnos3, recipe_cactbal3 }, per_player_gauges_only = true },
        { order = 53, code_run = { chip = recipe_cactbal3, count = 3 } },
    },
    instant = cacdanc.instant(2),
}
