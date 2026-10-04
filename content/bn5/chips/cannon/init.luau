--!strict
-- Cannon, HiCannon and M-Cannon: the cannon series (each chip's own comment above its definition).
-- Their records are written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local shooter = require("../lib/cannon")
local variants = require("../objects/projectile/variants")

-- Cannon (BN5 chip 0x001): BN6's cannon (@exelib/cannon, the original's action 0x14; BN5's the same code), firing
-- BN5's cannon shot.
local cannon = define.chip {
    id = "cannon",
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
    slot_in_limit = 3,
    action = shooter.action { id = "cannon/action", arm = shooter.arms.cannon, shot = variants.cannon },
}

-- HiCannon (BN5 chip 0x002): BN6's cannon (@exelib/cannon, the original's action 0x14; BN5's the same code),
-- firing BN5's cannon shot.
local hicannon = define.chip {
    id = "hicannon",
    codes = { "D", "E", "F" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 24,
    damage = 80,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = shooter.action { id = "hicannon/action", arm = shooter.arms.hicannon, shot = variants.cannon },
}

-- M-Cannon (BN5 chip 0x003): BN6's cannon (@exelib/cannon, the original's action 0x14; BN5's the same code),
-- firing BN5's cannon shot.
local m_cannon = define.chip {
    id = "m-cannon",
    codes = { "F", "G", "H" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 40,
    damage = 120,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = shooter.action { id = "m-cannon/action", arm = shooter.arms.m_cannon, shot = variants.cannon },
}

return { cannon = cannon, hicannon = hicannon, m_cannon = m_cannon }
