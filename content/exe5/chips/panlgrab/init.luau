--!strict
-- PanlGrab (EXE5 chip 0x074): a dimming that drops a grab shot on the first panel past the user's side's front in
-- its row: EXE6's (@exelib/grab; EXE5's the same code).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local grab = require("../lib/grab")

return define.chip {
    id = "panlgrab",
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
