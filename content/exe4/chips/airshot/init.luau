--!strict
-- AirShot (EXE4 chip 0x004; its record at 0x080197EC + 4 * 0x2C): the chips' action 0x23 (lib/airshot), its variant 0:
-- the air shooter (attachment row 9), the air shot (projectile row 4, its parameter +0x10), no recovery.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local airshot = require("../lib/airshot")
local collision = require("../rules/collision")
local projectile = require("../objects/projectile")
local sparks = require("../lib/sparks")

-- The air shot (projectile row 4, action 0x23's: 0x080ECBEE, its parameter +0x10): it pushes (hit modifier 0x21;
-- EXE5's row 4 0x61), with the impact spark.
local SHOT = projectile.variant {
    id = "airshot/shot",
    collision = { is = collision.pushing, hits = collision.hits_navis },
    hit_mod = 0x21,
    element = "null",
    spark = sparks.impact,
}

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
    action = airshot.action { id = "airshot/action", shooter = airshot.shooter, shot = SHOT, recovery = 0 },
}

return { airshot = chip }
