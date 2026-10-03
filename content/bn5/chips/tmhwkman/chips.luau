--!strict
-- TmhwkMan, TmhwkMnSP and TmhwkMnDS: the tmhwkman series (each chip's own comment above its definition).
-- Their records are written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("./navi")

-- TmhwkMan (BN5 chip 0x0fb): TomahawkMan cuts in and brings his tomahawk down on the two columns ahead (./navi).
local tmhwkman = define.chip {
    id = "bn5:tmhwkman",
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

-- TmhwkMnSP (BN5 chip 0x0fc): TomahawkMan cuts in and brings his tomahawk down on the two columns ahead
-- (../tmhwkman/navi); its damage by how fast its user deleted TomahawkMan SP (rules/sp-chips: formula 11, its table's
-- row 10).
local tmhwkmn_sp = define.chip {
    id = "bn5:tmhwkmn-sp",
    codes = { "T" },
    element = "wood",
    family = "wood",
    class = "mega",
    rarity = 3,
    mb = 81,
    damage = { formula = "sp_navi", slot = "sp/tmhwkman", by_time = { 280, 270, 260, 250, 240, 230, 220, 210, 200, 190, 180 }, per_player_gauges = 250 },
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon_sp,
}

-- TmhwkMnDS (BN5 chip 0x0fd): TomahawkMan, dark, cuts in and brings his tomahawk down on the two columns ahead
-- (../tmhwkman/navi); its damage by its user's side's statistic 3 (formula 33, its table's row 10).
local tmhwkmn_ds = define.chip {
    id = "bn5:tmhwkmn-ds",
    codes = { "T" },
    element = "wood",
    family = "wood",
    class = "mega",
    rarity = 3,
    mb = 81,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 180, 190, 200, 210, 220, 230, 240, 250, 260, 270, 280 }, per_player_gauges = 200 },
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_dark,
}

return { tmhwkman = tmhwkman, tmhwkmn_sp = tmhwkmn_sp, tmhwkmn_ds = tmhwkmn_ds }
