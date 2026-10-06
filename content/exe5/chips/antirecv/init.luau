--!strict
-- AntiRecv (EXE5 chip 0x093): a trap that turns the other side's next recovery into damage: EXE6's (@exelib/traps,
-- its counterattack the role kinds.anti_recovery; EXE5's the same code).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local traps = require("../lib/traps")

local chip: Chip = {
    codes = { "B", "J", "P" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 37,
    damage = 0,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 3,
    trap = "anti_recovery",
    dimming = traps.hook(),
}

return { antirecv = chip }
