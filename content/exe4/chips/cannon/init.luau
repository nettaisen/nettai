--!strict
-- Cannon, HiCannon and M-Cannon (EXE4 chips 0x001 to 0x003; their records at 0x080197EC + n * 0x2C): the cannons'
-- action 0x0B (lib/cannons), its variants 0 to 2: the cannon in palettes 0 to 2 (attachment rows 0 to 2), the
-- cannon's shot (projectile row 1).

local cannons = require("../lib/cannons")
local variants = require("../objects/projectile/variants")

local cannon: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 8,
    damage = 40,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    action = cannons.action { id = "cannon/action", arm = cannons.arm, shot = variants.cannon },
}

local hicannon: Chip = {
    codes = { "C", "D", "E" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 24,
    damage = 80,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    action = cannons.action { id = "hicannon/action", arm = cannons.arm_hi, shot = variants.cannon },
}

local m_cannon: Chip = {
    codes = { "E", "F", "G" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 40,
    damage = 120,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    action = cannons.action { id = "m-cannon/action", arm = cannons.arm_mega, shot = variants.cannon },
}

return { cannon = cannon, hicannon = hicannon, ["m-cannon"] = m_cannon }
