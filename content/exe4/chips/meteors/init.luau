--!strict
-- Meteors1, Meteors2 and Meteors3 (EXE4 chips 0x0A3 to 0x0A5; their records at 0x080197EC + n * 0x2C): the spawners'
-- action 0x20 (lib/spawners), its variant 0x11: five meteors on the target's panel in the user's row (./meteors).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local meteors = require("@self/meteors")
local spawners = require("../lib/spawners")

local meteors1: Chip = {
    action = spawners.action { id = "meteors1/action", spawn = meteors.spawn },
    codes = { "G", "L", "S", "*" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 1,
    mb = 15,
    damage = 50,
    hit_param = 10,
    lockout = 120,
    flags = { "has_damage", "library" },
}

local meteors2: Chip = {
    action = spawners.action { id = "meteors2/action", spawn = meteors.spawn },
    codes = { "M", "Q", "T" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 2,
    mb = 23,
    damage = 60,
    hit_param = 10,
    lockout = 120,
    flags = { "has_damage", "library" },
}

local meteors3: Chip = {
    action = spawners.action { id = "meteors3/action", spawn = meteors.spawn },
    codes = { "C", "R", "Z" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 3,
    mb = 31,
    damage = 70,
    hit_param = 10,
    lockout = 120,
    flags = { "has_damage", "library" },
}

return { meteors1 = meteors1, meteors2 = meteors2, meteors3 = meteors3 }
