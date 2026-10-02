--!strict
-- Boomer (BN5 chip 0x006): a boomerang flies from the user's back column along row 3, up the far column and back
-- along row 1: BN6's boomerang (@bn6/chips/boomer/boomerang, instant chip effect 1; BN5's the same code but that
-- it turns no panel to grass and has no tomahawk), with BN5's variant (0x080CAF50 row 0: half BN6's speeds) and
-- sprite (10-14).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local boomerang = require("@bn6/chips/boomer/boomerang")

local VARIANT = boomerang.variant { speed = 0x4_0000, turn_speed = 157286, grass = false, sprite = asset.sprite("bn5:boomerang") }

return define.chip {
    id = "bn5:boomer",
    codes = { "A", "L", "V", "*" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 2,
    mb = 16,
    damage = 60,
    hit_param = 10,
    lockout = 30,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    instant = boomerang.instant(VARIANT),
}
