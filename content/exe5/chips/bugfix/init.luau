--!strict
-- BugFix (EXE5 chip 0x0ce): a cut-in chip that fixes the user's NaviCust bugs while a copy of its navi glows over it:
-- EXE6's BugFix (the dimming handler's subtype 22) with EXE5's glow and no Beast Out (objects/bugfix).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local bugfix = require("../objects/bugfix")

local chip: Chip = {
    codes = { "H", "*" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 62,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library" },
    extra_flags = { 128 },
    slot_in_limit = 1,
    megaman = "light",
    dimming = bugfix.controller.spawn,
}

return { bugfix = chip }
