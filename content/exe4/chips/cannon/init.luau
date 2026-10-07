--!strict
-- Cannon, HiCannon and M-Cannon (EXE4 chips 0x001 to 0x003; their records at 0x080197EC + n * 0x2C): the cannons'
-- action 0x0B (lib/cannon), its variants 0 to 2: the cannon in palettes 0 to 2 (attachment rows 0 to 2), the
-- cannon's shot (projectile row 1).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local attachment = require("../objects/attachment")
local cannon_lib = require("../lib/cannon")
local collision = require("../rules/collision")
local projectile = require("../objects/projectile")
local sparks = require("../lib/sparks")

-- Their cannons (attachment rows 0 to 2: 0c-01 in palettes 0 to 2).
local CANNON_ARM = attachment.look { sprite = asset.sprite("cannon"), palette = 0 }
local HICANNON_ARM = attachment.look { sprite = asset.sprite("cannon"), palette = 1 }
local M_CANNON_ARM = attachment.look { sprite = asset.sprite("cannon"), palette = 2 }

-- The cannon's shot (projectile row 1, action 0x0B's variants 0 to 3: 0x080EB97A): it staggers (hit modifier 3), with
-- the impact spark.
local SHOT = projectile.variant {
    id = "cannon/shot",
    collision = { is = collision.attack, hits = collision.hits_navis },
    hit_mod = 3,
    element = "null",
    spark = sparks.impact,
}

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
    action = cannon_lib.action { id = "cannon/action", arm = CANNON_ARM, shot = SHOT },
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
    action = cannon_lib.action { id = "hicannon/action", arm = HICANNON_ARM, shot = SHOT },
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
    action = cannon_lib.action { id = "m-cannon/action", arm = M_CANNON_ARM, shot = SHOT },
}

return { cannon = cannon, hicannon = hicannon, ["m-cannon"] = m_cannon }
