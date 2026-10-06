--!strict
-- BodyGrd (EXE5 chip 0x15c): the Program Advance of AntiDmg, AntiNavi and Muramasa: an unseen striker drops ten
-- shurikens on the enemy navi, one every 20 ticks (./bodygrd: EXE5's controller sends out EXE6's striker at once,
-- where EXE6's BodyGrd is a trap).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local bodygrd = require("@self/bodygrd")
local recipe_antidmg = require("./antidmg").antidmg
local recipe_antinavi = require("./antinavi").antinavi
local recipe_muramasa = require("./muramasa").muramasa

local chip: Chip = {
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
    -- EXE5's recipes (written by tools/exe5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 25, sequence = { recipe_antidmg, recipe_antinavi, recipe_muramasa } },
    },
    dimming = bodygrd.hook(0),
    auto_battle = { class = "at_once" },
}

return { bodygrd = chip }
