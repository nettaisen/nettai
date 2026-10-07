--!strict
-- MokoRus1, MokoRus2 and MokoRus3 (EXE4 chips 0x082 to 0x084; their records at 0x080197EC + n * 0x2C): the spawners'
-- action 0x20 (lib/spawners), its variant 1: three Molokos charge along the rows (./moloko), their parameter (0 to 2)
-- their palette.
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local moloko = require("@self/moloko")
local spawners = require("../lib/spawners")

local mokorus1: Chip = {
    action = spawners.action { id = "mokorus1/action", spawn = moloko.spawner(0) },
    codes = { "C", "I", "M" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 14,
    damage = 60,
    hit_param = 20,
    lockout = 100,
    flags = { "has_damage", "standard_library", "library" },
}

local mokorus2: Chip = {
    action = spawners.action { id = "mokorus2/action", spawn = moloko.spawner(1) },
    codes = { "G", "K", "U" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 28,
    damage = 90,
    hit_param = 20,
    lockout = 100,
    flags = { "has_damage", "standard_library", "library" },
}

local mokorus3: Chip = {
    action = spawners.action { id = "mokorus3/action", spawn = moloko.spawner(2) },
    codes = { "O", "T", "Y" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 42,
    damage = 120,
    hit_param = 20,
    lockout = 100,
    flags = { "has_damage", "standard_library", "library" },
}

return { mokorus1 = mokorus1, mokorus2 = mokorus2, mokorus3 = mokorus3 }
