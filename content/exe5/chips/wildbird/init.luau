--!strict
-- WildBird (EXE5 chip 0x158): the Program Advance of two Katana1s and LarkManSP or LarkManDS: LarkMan cuts in and sends
-- his swoop round the field, faster and longer (../larkman/navi's WildBird: the summon table's entry 23, the swoop's
-- variant 6).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local navi = require("./larkman/navi")
local navi_chips = require("../lib/navi_chips/chip")
local recipe_katana1 = require("./katana").katana1
local recipe_larkman_ds = require("./larkman")["larkman-ds"]
local recipe_larkman_sp = require("./larkman")["larkman-sp"]

local chip: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 200,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    slot_in_limit = 1,
    -- EXE5's recipes (written by tools/exe5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 28, sequence = { recipe_katana1, recipe_katana1, recipe_larkman_sp } },
        { order = 29, sequence = { recipe_katana1, recipe_katana1, recipe_larkman_ds } },
    },
    navi = navi_chips.make {
        id = "wildbird/controller",
        summon = navi.summon_wild,
    },
    auto_battle = { class = "into_reach", range = "two_columns_ahead" },
}

return { wildbird = chip }
