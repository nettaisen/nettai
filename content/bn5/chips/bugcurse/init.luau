--!strict
-- BugCurse (BN5 chip 0x12f): a giga cut-in chip that lays a curse on the enemy navi, infecting it with bugs (./controller).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local controller = require("@self/controller")

return define.chip {
    id = "bugcurse",
    codes = { "C" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 73,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library" },
    extra_flags = { 4, 16 },
    slot_in_limit = 1,
    megaman = "dark",
    dimming = controller.hook,
}
