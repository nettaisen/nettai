--!strict
-- DrilArm3 (BN5 chip 0x04a): the navi's arm becomes a drill and two drills grind the two panels ahead for 24
-- ticks plus 10: BN6's DrilArm (action 0x4F, BN6's 0x2E) as BN5's (objects/drill).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local drill = require("../../objects/drill/drill")

return define.chip {
    id = "bn5:drilarm3",
    codes = { "I", "P", "Z" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 2,
    mb = 64,
    damage = 80,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = drill.action(false),
}
