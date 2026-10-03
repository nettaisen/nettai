--!strict
-- ColorPt (BN5 chip 0x07f): the user's front column goes to the other side, panel by panel, for Atk+10 a panel on
-- the chip at the hand's cursor: BN6's ColorPt with BN5's points (objects/colorpt).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local colorpt = require("../../objects/colorpt/colorpt")

return define.chip {
    id = "bn5:colorpt",
    codes = { "*" },
    element = "null",
    family = "plus",
    class = "standard",
    rarity = 1,
    mb = 25,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 1,
    dimming = colorpt.hook { palette = 0, bonus = 10 },
}
