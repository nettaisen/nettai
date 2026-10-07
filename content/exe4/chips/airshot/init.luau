--!strict
-- AirShot (EXE4 chip 0x004; its record at 0x080197EC + 4 * 0x2C): the chips' action 0x23 (lib/airshot), its variant 0:
-- the air shooter (attachment row 9), the air shot (projectile row 4, its parameter +0x10), no recovery.

local airshot = require("../lib/airshot")
local variants = require("../objects/projectile/variants")

local chip: Chip = {
    codes = { "A", "S", "V", "*" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 1,
    mb = 6,
    damage = 20,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    action = airshot.action { id = "airshot/action", shooter = airshot.shooter, shot = variants.air, recovery = 0 },
}

return { airshot = chip }
