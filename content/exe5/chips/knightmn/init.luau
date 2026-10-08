--!strict
-- KnightMn, KnigtMnSP and KnigtMnDS: the knightmn series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local navi = require("@self/navi")
local navi_chips = require("../lib/navi_chips/chip")
local sp_chips = require("../rules/sp_chips")

-- KnightMn (EXE5 chip 0x0fe): KnightMan cuts in and swings his wrecking ball around himself, confusing what it hits
-- (./navi).
local knightmn: Chip = {
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
    navi = navi_chips.make {
        id = "knightmn/controller",
        summon = navi.summon,
    },
    auto_battle = { class = "into_reach", range = "around" },
}

-- KnigtMnSP (EXE5 chip 0x0ff): KnightMan cuts in and swings his wrecking ball around himself, confusing what it hits
-- (../knightmn/navi); its damage by how fast its user deleted KnightMan SP (rules/sp_chips: formula 12, its table's row
-- 11).
local knigtmn_sp: Chip = {
    codes = { "K" },
    element = "null",
    family = "break",
    class = "mega",
    rarity = 3,
    mb = 90,
    sp = { by_time = { 320, 310, 300, 290, 280, 270, 260, 250, 240, 230, 220 }, operation_battle = 300 },
    damage = sp_chips.damage,
    hit_param = 178,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi_chips.make {
        id = "knigtmn-sp/controller",
        summon = navi.summon_sp,
    },
    auto_battle = { class = "into_reach", range = "around" },
}

-- KnigtMnDS (EXE5 chip 0x100): KnightMan, dark, cuts in and swings his wrecking ball around himself, confusing what it
-- hits (../knightmn/navi); its damage by its user's side's statistic 3 (formula 34, its table's row 11).
local knigtmn_ds: Chip = {
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
    navi = navi_chips.make {
        id = "knigtmn-ds/controller",
        summon = navi.summon_dark,
    },
    auto_battle = { class = "into_reach", range = "around" },
}

return { knightmn = knightmn, ["knigtmn-sp"] = knigtmn_sp, ["knigtmn-ds"] = knigtmn_ds }
