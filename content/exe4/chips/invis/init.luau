--!strict
-- Invis (EXE4 chip 0x085): its user turns invisible for six seconds: the dimming chips' action 0x0C, its variant 0x31
-- (0x080220E0's), EXE6's Invisibl (./controller), 360 ticks (the record's parameter). The opponent's Rush cancels it
-- (lib/supports/rush). Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local invisible = require("@self/controller")

local chip: Chip = {
    codes = { "*" },
    element = "null",
    family = "invisible",
    class = "standard",
    rarity = 2,
    mb = 12,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    extra_flags = { 2 },
    dimming = invisible.hook(360),
}

return { invis = chip }
