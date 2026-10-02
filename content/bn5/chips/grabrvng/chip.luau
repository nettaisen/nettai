--!strict
-- GrabRvng (BN5 chip 0x076): the user's side takes back the panels the other side stole, a hand dropping on every
-- enemy navi for each: BN6's controller (@bn6/chips/grabbnsh, GrabBnsh's and GrabRvng's). BN5's code is the same but
-- for the hand's effect number (0x29, BN6's 0x3B), which is the same effect (the grab shot's animation 1).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local controller = require("@bn6/chips/grabbnsh/controller")

return define.chip {
    id = "grabrvng",
    codes = { "F", "N", "P" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 48,
    damage = 40,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 1,
    dimming = controller.hook,
}
