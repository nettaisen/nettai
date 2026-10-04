--!strict
-- WindRack (BN5 chip 0x035): a swing of a rack that pushes the column ahead (chips/windrack/action: BN6's WindRack,
-- action 0x26, without its gusts).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local windrack = require("@self/action")

return define.chip {
    id = "windrack",
    codes = { "B", "F", "J", "*" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 2,
    mb = 23,
    damage = 100,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = windrack.action,
}
