--!strict
-- WideSht2 (BN5 chip 0x00a): the navi raises its wide shooter and sends a wave from the panel ahead over a column
-- of three: the shared WideSht (@common/widesht, the original's action 0x30, BN5's 0x25; the same code but for the
-- shooter's attachment row, BN5's 0x11 for BN6's 0x23, and the wave's height and sprite) with BN5's shooter (0c-2a,
-- the same place), its wave at the floor (BN6's at 16 pixels) and BN5's wide wave of kind 4 (objects/wave).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local wave = require("../../objects/wave/variants")
local widesht = require("../../objects/wave/wave")

return define.chip {
    id = "bn5:widesht2",
    codes = { "E", "F", "G" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 1,
    mb = 30,
    damage = 90,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = widesht.action { wave = wave.wide2, waves = 1, z = 0 },
}
