--!strict
-- TimeBom+ (BN5 chip 0x15b): the Program Advance of TimeBom1, TimeBom2 and TimeBom3 in that order: TimeBom1-3's bomb
-- with 300 HP, its own look and a harder shake (objects/timebom: BN6's TimeBom with BN5's placement).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local timebom = require("../../objects/timebom/timebom")
local recipe_timebom1 = require("../timebom1/chip")
local recipe_timebom2 = require("../timebom2/chip")
local recipe_timebom3 = require("../timebom3/chip")

return define.chip {
    id = "bn5:timebom",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 600,
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    slot_in_limit = 1,
    -- BN5's recipes (written by tools/bn5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 23, sequence = { recipe_timebom1, recipe_timebom2, recipe_timebom3 } },
    },
    dimming = timebom.controller.hook(timebom.variants.big),
}
