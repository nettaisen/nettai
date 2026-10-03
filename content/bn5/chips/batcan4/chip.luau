--!strict
-- BatCan4 (BN5 chip 0x13e): the navi raises its bat cannon and fires three unseen shots from the panel ahead, 16
-- ticks apart, the last staggering harder: BN6's BatCan4 (objects/batcan).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local batcan = require("../../objects/batcan/batcan")

return define.chip {
    id = "bn5:batcan4",
    codes = { "Z" },
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
    action = batcan.action { shots = { 1, 1, 3 } },
}
