--!strict
-- Counter1, Counter2 and Counter3 (EXE4 chips 0x03C to 0x03E; their records at 0x080197EC + n * 0x2C): the spawners'
-- action 0x20 (lib/spawners), its variant 0: a strike on the other side's navi wherever it is, if its counter window is
-- open (./strike).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local spawners = require("../lib/spawners")
local strike = require("@self/strike")

local counter1: Chip = {
    action = spawners.action { id = "counter1/action", spawn = strike.spawner },
    codes = { "F", "M", "T", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 14,
    damage = 70,
    hit_param = 50,
    lockout = 30,
    flags = { "has_damage", "standard_library", "library" },
}

local counter2: Chip = {
    action = spawners.action { id = "counter2/action", spawn = strike.spawner },
    codes = { "B", "H", "L", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 28,
    damage = 110,
    hit_param = 50,
    lockout = 30,
    flags = { "has_damage", "standard_library", "library" },
}

local counter3: Chip = {
    action = spawners.action { id = "counter3/action", spawn = strike.spawner },
    codes = { "A", "N", "V" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 42,
    damage = 150,
    hit_param = 50,
    lockout = 30,
    flags = { "has_damage", "standard_library", "library" },
}

return { counter1 = counter1, counter2 = counter2, counter3 = counter3 }
