--!strict
-- CsmoPris (BN5 chip 0x157): the Program Advance of two Astroid3s and CosmoMnSP or CosmoMnDS: CosmoMan cuts in, rises
-- over the middle row and sends nine comets along the rows that turn toward the enemy navi (./csmopris, the summon
-- table's entry 22).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local csmopris = require("./csmopris")
local recipe_astroid3 = require("../astroid/chips").astroid3
local recipe_cosmomn_ds = require("../cosmoman/chips").cosmomn_ds
local recipe_cosmomn_sp = require("../cosmoman/chips").cosmomn_sp

return define.chip {
    id = "csmopris",
    codes = {},
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 60,
    hit_param = 133,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    slot_in_limit = 1,
    -- BN5's recipes (written by tools/bn5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 26, sequence = { recipe_astroid3, recipe_astroid3, recipe_cosmomn_sp } },
        { order = 27, sequence = { recipe_astroid3, recipe_astroid3, recipe_cosmomn_ds } },
    },
    navi = csmopris.summon,
}
