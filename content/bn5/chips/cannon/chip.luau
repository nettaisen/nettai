--!strict
-- Cannon (BN5 chip 0x001): BN6's cannon (@common/cannon, the original's action 0x14; BN5's the same code), firing
-- BN5's cannon shot.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local shooter = require("../../lib/cannon")
local variants = require("../../objects/projectile/variants")

return define.chip {
    id = "bn5:cannon",
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
    action = shooter.action { id = "bn5:cannon/action", arm = shooter.arms.cannon, shot = variants.cannon },
}
