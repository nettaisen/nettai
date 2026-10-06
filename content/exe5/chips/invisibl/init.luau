--!strict
-- Invisibl (EXE5 chip 0x085): its user turns invisible for six seconds: EXE6's Invisibl (@exelib/invisibl, the
-- dimming spawner's entry 40; EXE5's the same code), 360 ticks (the record's parameter).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

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
    extra_flags = { 2, 128 },
    slot_in_limit = 3,
    dimming = invisible.hook(360),
}

return { invisibl = chip }
