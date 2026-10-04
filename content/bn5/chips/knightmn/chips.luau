--!strict
-- KnightMn, KnigtMnSP and KnigtMnDS: the knightmn series (each chip's own comment above its definition).
-- Their records are written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("./navi")

-- KnightMn (BN5 chip 0x0fe): KnightMan cuts in and swings his wrecking ball around himself, confusing what it hits
-- (./navi).
local knightmn = define.chip {
    id = "knightmn",
    codes = { "K" },
    element = "wood",
    family = "break",
    class = "mega",
    rarity = 2,
    mb = 50,
    damage = 170,
    hit_param = 178,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}

-- KnigtMnSP (BN5 chip 0x0ff): KnightMan cuts in and swings his wrecking ball around himself, confusing what it hits
-- (../knightmn/navi); its damage by how fast its user deleted KnightMan SP (rules/sp-chips: formula 12, its table's row
-- 11).
local knigtmn_sp = define.chip {
    id = "knigtmn-sp",
    codes = { "K" },
    element = "null",
    family = "break",
    class = "mega",
    rarity = 3,
    mb = 90,
    damage = { formula = "sp_navi", slot = "sp/knightmn", by_time = { 320, 310, 300, 290, 280, 270, 260, 250, 240, 230, 220 }, operation_battle = 300 },
    hit_param = 178,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon_sp,
}

-- KnigtMnDS (BN5 chip 0x100): KnightMan, dark, cuts in and swings his wrecking ball around himself, confusing what it
-- hits (../knightmn/navi); its damage by its user's side's statistic 3 (formula 34, its table's row 11).
local knigtmn_ds = define.chip {
    id = "knigtmn-ds",
    codes = { "K" },
    element = "null",
    family = "break",
    class = "mega",
    rarity = 3,
    mb = 90,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 220, 230, 240, 250, 260, 270, 280, 290, 300, 310, 320 }, operation_battle = 250 },
    hit_param = 178,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_dark,
}

return { knightmn = knightmn, knigtmn_sp = knigtmn_sp, knigtmn_ds = knigtmn_ds }
