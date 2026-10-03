--!strict
-- M-Cannon (BN5 chip 0x003): BN6's cannon (@bn6/lib/cannon, the original's action 0x14; BN5's the same code),
-- firing BN5's cannon shot.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local shooter = require("../../lib/cannon")
local variants = require("../../objects/projectile/variants")

return define.chip {
    id = "bn5:m-cannon",
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
    action = shooter.action { id = "bn5:m-cannon/action", arm = shooter.arms.m_cannon, shot = variants.cannon },
}
