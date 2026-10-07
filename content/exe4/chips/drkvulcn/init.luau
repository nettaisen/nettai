--!strict
-- DrkVulcn (EXE4 chip 0x0BE): a dark chip, which no folder holds and the custom screen offers (rules/dark_chips). The
-- vulcans' action 0x1F, its variant 4: the dark gun (attachment row 49) fires 24 shots, bullet row 14 (its parameter
-- +0x10): the panel found and the one behind it, modifier 1, the dark hit shown once. Its cost: 720 ticks of
-- confusion (rules/dark_chips).

local bullet = require("../objects/bullet")
local vulcans = require("../lib/vulcans")

local drkvulcn: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 4,
    mb = 99,
    damage = 20,
    hit_param = 2,
    lockout = 0,
    flags = { "has_damage", "dark" },
    action = vulcans.action {
        id = "drkvulcn/action",
        gun = vulcans.dark_gun,
        shots = 24,
        shot = bullet.variants.dark_vulcan,
    },
}

return { drkvulcn = drkvulcn }
