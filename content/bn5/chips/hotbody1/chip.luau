--!strict
-- HotBody1 (BN5 chip 0x09a): the navi bursts into flames that spread to the enemies around it, two rounds 30 ticks
-- apart (objects/hotbody, action 0x58; the record's second parameter, 2).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local hotbody = require("../../objects/hotbody/hotbody")

return define.chip {
    id = "bn5:hotbody1",
    codes = { "A", "F", "P" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 0,
    mb = 28,
    damage = 110,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = hotbody.action("bn5:hotbody1/action", 2),
}
