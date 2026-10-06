--!strict
-- NumbrMan, NumbrMnSP and NumbrMnDS: the numbrman series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local navi = require("@self/navi")

-- NumbrMan (EXE5 chip 0x0f2): NumberMan cuts in and throws a die three columns ahead, its face times his damage when it
-- bursts (./navi).
local numbrman = define.chip {
    id = "numbrman",
    codes = { "N" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 2,
    mb = 33,
    damage = 30,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}

-- NumbrMnSP (EXE5 chip 0x0f3): NumberMan cuts in and throws a die three columns ahead, its face times his damage when it
-- bursts (../numbrman/navi); its damage by how fast its user deleted NumberMan SP (rules/sp_chips: formula 10, its
-- table's row 9).
local numbrmn_sp = define.chip {
    id = "numbrmn-sp",
    codes = { "N" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 66,
    damage = { formula = "sp_navi", slot = "sp/numbrman", by_time = { 90, 85, 80, 75, 70, 65, 60, 55, 50, 45, 40 }, operation_battle = 70 },
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon_sp,
}

-- NumbrMnDS (EXE5 chip 0x0f4): NumberMan, dark, cuts in and throws a die three columns ahead, its face times his damage
-- when it bursts (../numbrman/navi); its damage by its user's side's statistic 3 (formula 32, its table's row 9).
local numbrmn_ds = define.chip {
    id = "numbrmn-ds",
    codes = { "N" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 66,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 40, 45, 50, 55, 60, 65, 70, 75, 80, 85, 90 }, operation_battle = 50 },
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_dark,
}

return { numbrman = numbrman, numbrmn_sp = numbrmn_sp, numbrmn_ds = numbrmn_ds }
