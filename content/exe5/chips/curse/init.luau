--!strict
-- Curse (EXE5 chip 0x174, no library chip; VDoll's curse's telop names it): a swing that throws a sonic boom down
-- the row, EXE5's action 0x2B (../sonicbom/action) with EXE5's sonic boom, hit modifier 0 (the record's first
-- parameter).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local sonicbom = require("./sonicbom/action")

local chip: Chip = {
    codes = { "E", "L", "S", "*" },
    element = "null",
    family = "sword",
    class = "special",
    rarity = 0,
    mb = 8,
    damage = 80,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    action = sonicbom.make(0, false),
}

return { curse = chip }
