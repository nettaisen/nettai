--!strict
-- Punisher (BN5 chip 0x175, no library chip; Guardian's strike back's telop names it): a swing that throws a sonic
-- boom down the row, BN5's action 0x2B (../sonicbom/action) with BN5's sonic boom, hit modifier 0 (the record's
-- first parameter).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local sonicbom = require("../sonicbom/action")

return define.chip {
    id = "punisher",
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
    action = sonicbom.make("punisher/action", 0, false),
}
