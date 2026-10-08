--!strict
-- Colonel, ColonelSP and ColonelDS: the colonel series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local navi = require("@self/navi")
local navi_chips = require("../lib/navi_chips/chip")
local sp_chips = require("../rules/sp_chips")

-- Colonel (EXE5 chip 0x0f5): Colonel cuts in and divides the screen with a diagonal slash on the first enemy navi ahead
-- (./navi).
local colonel: Chip = {
    codes = { "C" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 2,
    mb = 39,
    damage = 160,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi_chips.make {
        id = "colonel/controller",
        summon = navi.summon { palette = 0, sword_palette = 0, strike = navi.screen_divide },
    },
    auto_battle = { class = "into_reach", range = "ahead_in_row" },
}

-- ColonelSP (EXE5 chip 0x0f6): Colonel cuts in and divides the screen with a diagonal slash on the first enemy navi
-- ahead (../colonel/navi); its damage by how fast its user deleted Colonel SP (rules/sp_chips: formula 8, its table's
-- row 7).
local colonel_sp: Chip = {
    codes = { "C" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 75,
    sp = { by_time = { 300, 290, 280, 270, 260, 250, 240, 230, 220, 210, 200 }, operation_battle = 230 },
    damage = sp_chips.damage,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi_chips.make {
        id = "colonel-sp/controller",
        summon = navi.summon { palette = 0, sword_palette = 0, strike = navi.screen_divide },
    },
    auto_battle = { class = "into_reach", range = "ahead_in_row" },
}

-- ColonelDS (EXE5 chip 0x0f7): Colonel, dark, cuts in and divides the screen with a diagonal slash on the first enemy
-- navi ahead (../colonel/navi); its damage by its user's side's statistic 3 (formula 30, its table's row 7).
local colonel_ds: Chip = {
    codes = { "C" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 75,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 200, 210, 220, 230, 240, 250, 260, 270, 280, 290, 300 }, operation_battle = 200 },
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi_chips.make {
        id = "colonel-ds/controller",
        summon = navi.summon { palette = 6, sword_palette = 6, strike = navi.screen_divide },
    },
    auto_battle = { class = "into_reach", range = "ahead_in_row" },
}

return { colonel = colonel, ["colonel-sp"] = colonel_sp, ["colonel-ds"] = colonel_ds }
