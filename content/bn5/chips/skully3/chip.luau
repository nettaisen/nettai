--!strict
-- Skully3 (BN5 chip 0x047): skulls that hop toward the nearest enemy, confusing what they hit (BN5's Skully,
-- objects/skully: action 0x55).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local skully = require("../../objects/skully/skully")

return define.chip {
    id = "bn5:skully3",
    codes = { "G", "N", "U" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 76,
    damage = 140,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = skully.action("bn5:skully3/action", skully.skulls(2)),
}
