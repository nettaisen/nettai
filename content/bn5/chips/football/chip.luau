--!strict
-- Football (BN5 chip 0x159): the Program Advance of two MrkCan2s and GridMnSP or GridMnDS: GridMan cuts in and kicks
-- four balls that come down over blocks of the enemy's area (./football, the summon table's entry 24).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local football = require("./football")
local recipe_gridman_ds = require("../gridman-ds/chip")
local recipe_gridman_sp = require("../gridman-sp/chip")
local recipe_mrkcan2 = require("../mrkcan2/chip")

return define.chip {
    id = "bn5:football",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 180,
    hit_param = 143,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    slot_in_limit = 1,
    -- BN5's recipes (written by tools/bn5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 30, sequence = { recipe_mrkcan2, recipe_mrkcan2, recipe_gridman_sp } },
        { order = 31, sequence = { recipe_mrkcan2, recipe_mrkcan2, recipe_gridman_ds } },
    },
    navi = football.summon,
}
