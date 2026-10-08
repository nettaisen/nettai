--!strict
-- CsmoPris (EXE5 chip 0x157): the Program Advance of two Astroid3s and CosmoMnSP or CosmoMnDS: CosmoMan cuts in, rises
-- over the middle row and sends nine comets along the rows that turn toward the enemy navi (./csmopris, the summon
-- table's entry 22).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local csmopris = require("@self/csmopris")
local navi_chips = require("../lib/navi_chips/chip")
local recipe_astroid3 = require("./astroid").astroid3
local recipe_cosmomn_ds = require("./cosmoman")["cosmomn-ds"]
local recipe_cosmomn_sp = require("./cosmoman")["cosmomn-sp"]

local chip: Chip = {
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
    -- EXE5's recipes (written by tools/exe5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 26, sequence = { recipe_astroid3, recipe_astroid3, recipe_cosmomn_sp } },
        { order = 27, sequence = { recipe_astroid3, recipe_astroid3, recipe_cosmomn_ds } },
    },
    navi = navi_chips.make {
        id = "csmopris/controller",
        summon = csmopris.summon,
    },
    auto_battle = { class = "at_once" },
}

return { csmopris = chip }
