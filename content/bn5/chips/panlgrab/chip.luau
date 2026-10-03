--!strict
-- PanlGrab (BN5 chip 0x074): a dimming that drops a grab shot on the first panel past the user's side's front in
-- its row: BN6's (@bn6/lib/grab; BN5's the same code).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local grab = require("@bn6/lib/grab/controller")

return define.chip {
    id = "bn5:panlgrab",
    codes = { "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 6,
    damage = 10,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 1,
    dimming = grab.panel,
}
