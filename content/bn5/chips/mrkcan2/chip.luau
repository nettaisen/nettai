--!strict
-- MrkCan2 (BN5 chip 0x00d): a sight sweeps the field ahead and locks on to the first enemy it finds, paralyzing it,
-- and the cannon fires at it (BN5's own MarkCannon, objects/mrkcan: action 0x4E).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local mrkcan = require("../../objects/mrkcan/mrkcan")

return define.chip {
    id = "bn5:mrkcan2",
    codes = { "D", "F", "Z" },
    element = "null",
    family = "cursor",
    class = "standard",
    rarity = 1,
    mb = 27,
    damage = 100,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = mrkcan.action { id = "bn5:mrkcan2/action", cannon = mrkcan.cannon(3) },
}
