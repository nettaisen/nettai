--!strict
-- LCounter (EXE5 chip 0x120): a Liberation chip: a copy of the user's navi glows over it as it logs in, and the side gets
-- five uses of Counter's Liberation ability, which only a Liberation Mission's specials use (the dimming handler's
-- subtype 66, objects/bugfix).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local bugfix = require("../objects/bugfix")

local chip: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 4,
    mb = 80,
    damage = 180,
    hit_param = 20,
    lockout = 0,
    flags = { "dimming" },
    slot_in_limit = 1,
    dimming = bugfix.liberation("counter"),
}

return { lcounter = chip }
