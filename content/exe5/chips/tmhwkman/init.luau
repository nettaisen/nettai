--!strict
-- TmhwkMan, TmhwkMnSP and TmhwkMnDS: the tmhwkman series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local navi = require("@self/navi")
local sp_chips = require("../rules/sp_chips")

-- TmhwkMan (EXE5 chip 0x0fb): TomahawkMan cuts in and brings his tomahawk down on the two columns ahead (./navi).
local tmhwkman: Chip = {
    codes = { "T" },
    element = "wood",
    family = "wood",
    class = "mega",
    rarity = 2,
    mb = 43,
    damage = 140,
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}

-- TmhwkMnSP (EXE5 chip 0x0fc): TomahawkMan cuts in and brings his tomahawk down on the two columns ahead
-- (../tmhwkman/navi); its damage by how fast its user deleted TomahawkMan SP (rules/sp_chips: formula 11, its table's
-- row 10).
local tmhwkmn_sp: Chip = {
    codes = { "T" },
    element = "wood",
    family = "wood",
    class = "mega",
    rarity = 3,
    mb = 81,
    sp = { by_time = { 280, 270, 260, 250, 240, 230, 220, 210, 200, 190, 180 }, operation_battle = 250 },
    damage = sp_chips.damage,
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon_sp,
}

-- TmhwkMnDS (EXE5 chip 0x0fd): TomahawkMan, dark, cuts in and brings his tomahawk down on the two columns ahead
-- (../tmhwkman/navi); its damage by its user's side's statistic 3 (formula 33, its table's row 10).
local tmhwkmn_ds: Chip = {
    codes = { "T" },
    element = "wood",
    family = "wood",
    class = "mega",
    rarity = 3,
    mb = 81,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 180, 190, 200, 210, 220, 230, 240, 250, 260, 270, 280 }, operation_battle = 200 },
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_dark,
}

return { tmhwkman = tmhwkman, ["tmhwkmn-sp"] = tmhwkmn_sp, ["tmhwkmn-ds"] = tmhwkmn_ds }
