--!strict
-- BatCan3 (BN5 chip 0x13d): the navi raises its bat cannon and fires an unseen shot from the panel ahead on the 7th
-- tick: BN6's BatCan (objects/batcan).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local batcan = require("../../objects/batcan/batcan")

return define.chip {
    id = "bn5:batcan3",
    codes = { "R" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 5,
    damage = 40,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage" },
    slot_in_limit = 1,
    action = batcan.action { shots = { 3 } },
}
