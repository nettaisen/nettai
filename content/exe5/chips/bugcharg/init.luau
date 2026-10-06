--!strict
-- BugCharg (EXE5 chip 0x136): a giga cut-in chip whose stand-in takes in its user's NaviCust bugs and fires a heavy shot for each (./controller).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local controller = require("@self/controller")

local chip: Chip = {
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
    auto_battle = { class = "into_reach", range = "ahead_in_row" },
}

return { bugcharg = chip }
