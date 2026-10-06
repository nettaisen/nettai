--!strict
-- DrkRecov (EXE5 chip 0x0c2): the navi heals 1000 HP in the tick after it uses it: EXE6's heal
-- (@exelib/recov/heal, the original's action 0x17, EXE6's 0x20; EXE5's 0x080EC484 the same heal and table,
-- 0x080EC4B8) and, unless an AntiRecv turned it, the side's mood gains the use's +0x0A, the record's parameter (0).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local heal = require("@exelib/recov/heal")

local chip: Chip = {
    codes = { "H", "*" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 4,
    mb = 99,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dark" },
    slot_in_limit = 1,
    action = heal.action { hp = 1000, mood = 0 },
}

return { drkrecov = chip }
