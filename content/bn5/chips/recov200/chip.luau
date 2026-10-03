--!strict
-- Recov200 (BN5 chip 0x071): the navi heals 200 HP in the tick after it uses it: BN6's heal (@bn6/chips/recov/heal,
-- the original's action 0x17, BN6's 0x20; BN5's 0x080EC484 the same heal and table, 0x080EC4B8) and, unless an
-- AntiRecv turned it, the side's mood gains the use's +0x0A, the record's parameter (50).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local heal = require("@common/recov/heal")

return define.chip {
    id = "bn5:recov200",
    codes = { "I" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 2,
    mb = 56,
    damage = 0,
    hit_param = 50,
    lockout = 30,
    flags = { "standard_library", "library" },
    slot_in_limit = 1,
    action = heal.action { hp = 200, mood = 50 },
}
