--!strict
-- BugCharg (BN5 chip 0x136): a giga cut-in chip whose stand-in takes in its user's NaviCust bugs and fires a heavy shot for each (./controller).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local controller = require("./controller")

return define.chip {
    id = "bugcharg",
    codes = { "B" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 77,
    damage = 200,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage" },
    extra_flags = { 8, 16 },
    slot_in_limit = 1,
    megaman = "dark",
    dimming = controller.hook,
}
