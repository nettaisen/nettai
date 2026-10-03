--!strict
-- HiCannon (BN5 chip 0x002): BN6's cannon (@bn6/lib/cannon, the original's action 0x14; BN5's the same code),
-- firing BN5's cannon shot.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local shooter = require("@bn6/lib/cannon")
local variants = require("../../objects/projectile/variants")

return define.chip {
    id = "bn5:hicannon",
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
    action = shooter.action { id = "bn5:hicannon/action", arm = shooter.arms.hicannon, shot = variants.cannon },
}
