--!strict
-- ElemRage (BN5 chip 0x080): a row of three flames from the panel ahead, of the element of the panel the user stands
-- on, which then spread and paralyze (objects/elemrage, action 0x56; the record's parameters: 3 flames, 30 ticks).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local elemrage = require("../../objects/elemrage/elemrage")

return define.chip {
    id = "bn5:elemrage",
    codes = { "E", "K", "W" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 39,
    damage = 90,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = elemrage.action("bn5:elemrage/action", 3, 30, 0),
}
