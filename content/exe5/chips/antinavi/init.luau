--!strict
-- AntiNavi (EXE5 chip 0x090): a trap that turns the other side's next navi chip around: EXE6's (@exelib/traps; EXE5's
-- the same code).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local traps = require("../lib/traps")

return define.chip {
    id = "antinavi",
    codes = { "M", "T", "V" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 44,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 3,
    trap = "anti_navi",
    dimming = traps.hook(),
}
