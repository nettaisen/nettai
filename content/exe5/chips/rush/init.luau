--!strict
-- Rush (EXE5 chip 0x179, past the library): the chip the NaviCust supports' telop names as Rush comes (the role
-- `chips.rush`; 0x0800E498 passes it to the supports' controller, lib/supports). Its record's use is the cannon's
-- (action 0x14, subtype 0), which nothing starts: no folder or hand holds it.
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local cannon_chips = require("./cannon")

return define.chip {
    id = "rush",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "special",
    rarity = 0,
    mb = 5,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = {},
    slot_in_limit = 1,
    traits = { "hides_advance_code" },
    action = cannon_chips.cannon.action,
}
