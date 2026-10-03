--!strict
-- WideSht3 (BN5 chip 0x00b): the navi raises its wide shooter and sends a wave from the panel ahead over a column
-- of three: BN6's WideSht (@bn6/chips/widesht, the original's action 0x30, BN5's 0x25; the same code but for the
-- shooter's attachment row, BN5's 0x11 for BN6's 0x23, and the wave's height and sprite) with BN5's shooter (0c-2a,
-- the same place), its wave at the floor (BN6's at 16 pixels) and BN5's wide wave of kind 5 (objects/wave).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local attachment = require("@bn6/objects/attachment/attachment")
local wave = require("../../objects/wave/variants")
local widesht = require("@bn6/chips/widesht/action")

-- BN5's wide shooter (its attachment row 0x11).
local SHOOTER = attachment.look { sprite = asset.sprite("bn5:wide-shooter"), palette = 0, attach_point = 12 }

return define.chip {
    id = "bn5:widesht3",
    codes = { "S", "T", "U" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 2,
    mb = 50,
    damage = 110,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = widesht.action { wave = wave.wide3, waves = 1, shooter = SHOOTER, z = 0 },
}
