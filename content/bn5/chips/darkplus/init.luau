--!strict
-- DarkPlus (BN5 chip 0x0be): a dark chip: its damage becomes the Atk+ bonus of the hand's next chip, if that does
-- damage, and its navi takes the dark tint (lib/plus: action 0x1A's effect 3 with its second parameter set). Its use
-- costs a step bug (rules/light-dark).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local plus = require("../lib/plus")

return define.chip {
    id = "darkplus",
    codes = { "*" },
    element = "null",
    family = "plus",
    class = "standard",
    rarity = 4,
    mb = 99,
    damage = 50,
    hit_param = 0,
    lockout = 20,
    flags = { "dark" },
    slot_in_limit = 1,
    instant = plus.dark,
}
