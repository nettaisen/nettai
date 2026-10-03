--!strict
-- Astroid3 (BN5 chip 0x06a): a shower of 10 Fire meteors, one every 20 ticks on one of the enemy navis' panels or
-- beside it, each cracking its panel (objects/meteors: instant chip effect 17, its second parameter 10).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local meteors = require("../../objects/meteors/meteors")

return define.chip {
    id = "bn5:astroid3",
    codes = { "C", "P", "Y" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 2,
    mb = 56,
    damage = 80,
    hit_param = 138,
    lockout = 60,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    instant = meteors.instant(false, 10),
}
