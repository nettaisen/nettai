--!strict
-- Skully2 (BN5 chip 0x046): skulls that hop toward the nearest enemy, confusing what they hit (BN5's Skully,
-- objects/skully: action 0x55).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local skully = require("../../objects/skully/skully")

return define.chip {
    id = "bn5:skully2",
    codes = { "A", "H", "V" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 50,
    damage = 120,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = skully.action("bn5:skully2/action", skully.skulls(1)),
}
