--!strict
-- GrabBnsh (BN5 chip 0x077): the user's side takes back the panels the other side stole, a hand dropping on every
-- enemy navi for each: BN6's controller (@bn6/chips/grabbnsh; BN5's the same code).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local controller = require("@bn6/chips/grabbnsh/controller")

return define.chip {
    id = "grabbnsh",
    codes = { "D", "L", "R" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 24,
    damage = 20,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 1,
    dimming = controller.hook,
}
