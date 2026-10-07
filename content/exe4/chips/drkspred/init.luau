--!strict
-- DrkSpred (EXE4 chip 0x0C0): a dark chip, which no folder holds and the custom screen offers (rules/dark_chips).
-- The spreaders' action 0x1E, its variant 3: the dark gun (attachment row 0x32) fires on the second tick, at height
-- 0x10, bullet row 15 (its parameter +0x10): a dark burst over the 3x3 around the first thing it finds, modifier 1.
-- Its cost: a step leaves a poison panel (rules/dark_chips).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local bullet = require("../objects/bullet")
local spreaders = require("../lib/spreaders")

local drkspred: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 4,
    mb = 99,
    damage = 400,
    hit_param = 50,
    lockout = 0,
    flags = { "has_damage", "dark" },
    action = spreaders.action {
        id = "drkspred/action",
        gun = spreaders.dark_gun,
        delay = 1,
        z = 0x10,
        shot = bullet.variants.dark_burst,
    },
}

return { drkspred = drkspred }
