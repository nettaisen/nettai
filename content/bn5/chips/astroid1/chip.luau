--!strict
-- Astroid1 (BN5 chip 0x068): a shower of 6 Fire meteors, one every 20 ticks on one of the enemy navis' panels or
-- beside it, each cracking its panel (objects/meteors: instant chip effect 17, its second parameter 6).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local meteors = require("../../objects/meteors/meteors")

return define.chip {
    id = "bn5:astroid1",
    codes = { "A", "Q", "T" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 0,
    mb = 26,
    damage = 60,
    hit_param = 138,
    lockout = 60,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    instant = meteors.instant(false, 6),
}
