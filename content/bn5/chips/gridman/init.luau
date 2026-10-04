--!strict
-- GridMan, GridManSP and GridManDS: the gridman series (each chip's own comment above its definition).
-- Their records are written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("@self/navi")

-- GridMan (BN5 chip 0x113): GridMan cuts in, dashes across the field and strikes the whole enemy area (./navi).
local gridman = define.chip {
    id = "gridman",
    codes = { "F" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 2,
    mb = 46,
    damage = 100,
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}

-- GridManSP (BN5 chip 0x114): GridMan cuts in, dashes across the field and strikes the whole enemy area (../gridman/navi);
-- its damage by how fast its user deleted GridMan SP (rules/sp-chips: formula 19, its table's row 18).
local gridman_sp = define.chip {
    id = "gridman-sp",
    codes = { "F" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 72,
    damage = { formula = "sp_navi", slot = "sp/gridman", by_time = { 250, 240, 230, 220, 210, 200, 190, 180, 170, 160, 150 }, operation_battle = 230 },
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}

-- GridManDS (BN5 chip 0x115): GridMan, dark, cuts in, dashes across the field and strikes the whole enemy area
-- (../gridman/navi); its damage by its user's side's statistic 3 (formula 41, its table's row 18).
local gridman_ds = define.chip {
    id = "gridman-ds",
    codes = { "F" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 72,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 150, 160, 170, 180, 190, 200, 210, 220, 230, 240, 250 }, operation_battle = 180 },
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_dark,
}

return { gridman = gridman, gridman_sp = gridman_sp, gridman_ds = gridman_ds }
