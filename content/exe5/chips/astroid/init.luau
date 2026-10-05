--!strict
-- Astroid1, Astroid2 and Astroid3: the astroid series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local meteors = require("./meteors/meteors")

-- Astroid1 (EXE5 chip 0x068): a shower of 6 Fire meteors, one every 20 ticks on one of the enemy navis' panels or
-- beside it, each cracking its panel (chips/meteors: instant chip effect 17, its second parameter 6).
local astroid1 = define.chip {
    id = "astroid1",
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

-- Astroid2 (EXE5 chip 0x069): a shower of 8 Fire meteors, one every 20 ticks on one of the enemy navis' panels or
-- beside it, each cracking its panel (chips/meteors: instant chip effect 17, its second parameter 8).
local astroid2 = define.chip {
    id = "astroid2",
    codes = { "E", "M", "U" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 1,
    mb = 42,
    damage = 70,
    hit_param = 138,
    lockout = 60,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    instant = meteors.instant(false, 8),
}

-- Astroid3 (EXE5 chip 0x06a): a shower of 10 Fire meteors, one every 20 ticks on one of the enemy navis' panels or
-- beside it, each cracking its panel (chips/meteors: instant chip effect 17, its second parameter 10).
local astroid3 = define.chip {
    id = "astroid3",
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

return { astroid1 = astroid1, astroid2 = astroid2, astroid3 = astroid3 }
