--!strict
-- DarkMetr (BN5 chip 0x0c3): a dark chip: a shower of Fire meteors, 10 and 2 more for each of the first three
-- reactions its side's navi took, one every 10 ticks on one of the enemy navis' panels, each breaking its panel and
-- carrying the bug 0x19 (chips/meteors: instant chip effect 17 with its first parameter set). Its use costs custom
-- drain (rules/light-dark).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local meteors = require("./meteors/meteors")

return define.chip {
    id = "darkmetr",
    codes = { "F" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 4,
    mb = 99,
    damage = 100,
    hit_param = 132,
    lockout = 20,
    flags = { "has_damage", "dark" },
    slot_in_limit = 1,
    instant = meteors.instant(true, 0),
}
