--!strict
-- AirShot (BN5 chip 0x004): a shot from the panel ahead that pushes what it hits: the shared AirShot
-- (@exelib/airshot, BN6's action 0x21, BN5's 0x1D; the same code but for the shooter's attachment row, BN5's 8,
-- BN6's 0x13, the same look), with BN5's air shooter, attachment, projectile and shot (no wind element: BN5's variant
-- row 4); its sound BN6's.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local airshot = require("@exelib/airshot")
local attachment = require("../../objects/attachment/attachment")
local projectile = require("../../lib/projectile")
local variants = require("../../objects/projectile/variants")

-- BN5's air shooter (its attachment row 8: BN6's row 0x13's look, BN5's sprite).
local SHOOTER = attachment.look { sprite = asset.sprite("bn5:air-shooter"), palette = 0, attach_point = 10 }

return define.chip {
    id = "bn5:airshot",
    codes = { "*" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 1,
    mb = 6,
    damage = 20,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 3,
    action = airshot.action(
        { shooter = SHOOTER, shot = variants.air },
        { sound = asset.sound("bn6:h-burst"), attachment = attachment.kind, projectiles = projectile }
    ),
}
