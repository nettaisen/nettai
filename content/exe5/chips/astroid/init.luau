--!strict
-- Astroid1, Astroid2 and Astroid3: the astroid series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local collision = require("../rules/collision")
local falling_meteor = require("@exelib/meteors/falling_meteor")
local meteors = require("./meteors/meteors")

-- Their shower: a meteor every 20 ticks, on the navi picked or beside it, each cracking its panel (the meteor's
-- row 3).
local SHOWER = meteors.variant {
    period = 20,
    row = falling_meteor.row {
        is = collision.thrown,
        hits = collision.hits_navis,
        hit_mod = 3,
        shake = 1,
        panel = "crack",
    },
    aim = meteors.aim,
}

-- Astroid1 (EXE5 chip 0x068): a shower of 6 Fire meteors, one every 20 ticks on one of the enemy navis' panels or
-- beside it, each cracking its panel (chips/meteors: instant chip effect 17, its second parameter 6).
local astroid1: Chip = {
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
    instant = meteors.instant(SHOWER, 6),
    auto_battle = { class = "at_once" },
}

-- Astroid2 (EXE5 chip 0x069): a shower of 8 Fire meteors, one every 20 ticks on one of the enemy navis' panels or
-- beside it, each cracking its panel (chips/meteors: instant chip effect 17, its second parameter 8).
local astroid2: Chip = {
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
    instant = meteors.instant(SHOWER, 8),
    auto_battle = { class = "at_once" },
}

-- Astroid3 (EXE5 chip 0x06a): a shower of 10 Fire meteors, one every 20 ticks on one of the enemy navis' panels or
-- beside it, each cracking its panel (chips/meteors: instant chip effect 17, its second parameter 10).
local astroid3: Chip = {
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
    instant = meteors.instant(SHOWER, 10),
    auto_battle = { class = "at_once" },
}

return { astroid1 = astroid1, astroid2 = astroid2, astroid3 = astroid3 }
