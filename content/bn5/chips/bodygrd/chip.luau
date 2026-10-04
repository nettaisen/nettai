--!strict
-- BodyGrd (BN5 chip 0x15c): the Program Advance of AntiDmg, AntiNavi and Muramasa: an unseen striker drops ten
-- shurikens on the enemy navi, one every 20 ticks (./bodygrd: BN5's controller sends out BN6's striker at once,
-- where BN6's BodyGrd is a trap).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local bodygrd = require("./bodygrd")
local recipe_antidmg = require("../antidmg/chip")
local recipe_antinavi = require("../antinavi/chip")
local recipe_muramasa = require("../muramasa/chip")

return define.chip {
    id = "bodygrd",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 70,
    hit_param = 133,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    slot_in_limit = 1,
    -- BN5's recipes (written by tools/bn5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 25, sequence = { recipe_antidmg, recipe_antinavi, recipe_muramasa } },
    },
    dimming = bodygrd.hook(0),
}
