--!strict
-- GreenWd1, GreenWd2 and GreenWd3 (EXE4 chips 0x0B8 to 0x0BA; their records at 0x080197EC + n * 0x2C): the spawners'
-- action 0x20 (lib/spawners), its variant 0x0C, their parameter 1 (lib/towers): a wood tower rises from each grass
-- panel of the other side's holding its navi, a column at a time.
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local spawners = require("../lib/spawners")
local towers = require("../lib/towers")

local RAISE = towers.spawner(1)

local greenwd1: Chip = {
    action = spawners.action { id = "greenwd1/action", spawn = RAISE },
    codes = { "G", "J", "Y" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 1,
    mb = 35,
    damage = 120,
    hit_param = 30,
    lockout = 45,
    flags = { "has_damage", "library" },
}

local greenwd2: Chip = {
    action = spawners.action { id = "greenwd2/action", spawn = RAISE },
    codes = { "D", "K", "P" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 2,
    mb = 40,
    damage = 140,
    hit_param = 30,
    lockout = 45,
    flags = { "has_damage", "library" },
}

local greenwd3: Chip = {
    action = spawners.action { id = "greenwd3/action", spawn = RAISE },
    codes = { "E", "N", "P" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 3,
    mb = 45,
    damage = 160,
    hit_param = 30,
    lockout = 45,
    flags = { "has_damage", "library" },
}

return { greenwd1 = greenwd1, greenwd2 = greenwd2, greenwd3 = greenwd3 }
