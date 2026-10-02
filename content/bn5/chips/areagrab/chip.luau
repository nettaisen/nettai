--!strict
-- AreaGrab (BN5 chip 0x075): a dimming that drops grab shots on the first column in front not wholly the user's
-- side's: BN6's (@bn6/lib/grab; BN5's the same code).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local grab = require("@bn6/lib/grab/controller")

return define.chip {
    id = "areagrab",
    codes = { "A", "L", "S", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 8,
    damage = 10,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 1,
    dimming = grab.area,
}
