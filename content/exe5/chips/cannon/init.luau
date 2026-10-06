--!strict
-- Cannon, HiCannon and M-Cannon: the cannon series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local shooter = require("../lib/cannon")
local variants = require("../objects/projectile/variants")

-- Cannon (EXE5 chip 0x001): EXE6's cannon (@exelib/cannon, the original's action 0x14; EXE5's the same code), firing
-- EXE5's cannon shot.
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
    slot_in_limit = 3,
    action = shooter.action { arm = shooter.arms.cannon, shot = variants.cannon },
    auto_battle = { class = "into_reach", range = "ahead_in_row" },
}

-- HiCannon (EXE5 chip 0x002): EXE6's cannon (@exelib/cannon, the original's action 0x14; EXE5's the same code),
-- firing EXE5's cannon shot.
local hicannon: Chip = {
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
    action = shooter.action { arm = shooter.arms.hicannon, shot = variants.cannon },
    auto_battle = { class = "into_reach", range = "ahead_in_row" },
}

-- M-Cannon (EXE5 chip 0x003): EXE6's cannon (@exelib/cannon, the original's action 0x14; EXE5's the same code),
-- firing EXE5's cannon shot.
local m_cannon: Chip = {
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
    action = shooter.action { arm = shooter.arms.m_cannon, shot = variants.cannon },
    auto_battle = { class = "into_reach", range = "ahead_in_row" },
}

return { cannon = cannon, hicannon = hicannon, ["m-cannon"] = m_cannon }
