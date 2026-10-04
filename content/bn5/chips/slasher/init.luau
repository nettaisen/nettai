--!strict
-- Slasher (BN5 chip 0x038): while A is held, a wide slash at the column of an enemy navi that steps onto the user's
-- area (chips/slasher/action: BN5's own action 0x29).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local slasher = require("@self/action")

return define.chip {
    id = "slasher",
    codes = { "B", "I", "R" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 2,
    mb = 18,
    damage = 240,
    hit_param = 50,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = slasher.action,
}
