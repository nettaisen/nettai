--!strict
-- AquaUp1, AquaUp2 and AquaUp3 (EXE4 chips 0x0B5 to 0x0B7; their records at 0x080197EC + n * 0x2C): the spawners'
-- action 0x20 (lib/spawners), its variant 0x0C, their parameter 0 (lib/towers): an aqua tower rises from each cracked
-- panel of the other side's holding its navi, a column at a time.
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local spawners = require("../lib/spawners")
local towers = require("../lib/towers")

local RAISE = towers.spawner(0)

local aquaup1: Chip = {
    action = spawners.action { id = "aquaup1/action", spawn = RAISE },
    codes = { "B", "S", "Z" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 1,
    mb = 22,
    damage = 110,
    hit_param = 30,
    lockout = 45,
    flags = { "has_damage", "library" },
}

local aquaup2: Chip = {
    action = spawners.action { id = "aquaup2/action", spawn = RAISE },
    codes = { "I", "P", "U" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 2,
    mb = 28,
    damage = 140,
    hit_param = 30,
    lockout = 45,
    flags = { "has_damage", "library" },
}

local aquaup3: Chip = {
    action = spawners.action { id = "aquaup3/action", spawn = RAISE },
    codes = { "F", "K", "W" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 3,
    mb = 34,
    damage = 170,
    hit_param = 30,
    lockout = 45,
    flags = { "has_damage", "library" },
}

return { aquaup1 = aquaup1, aquaup2 = aquaup2, aquaup3 = aquaup3 }
