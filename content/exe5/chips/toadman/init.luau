--!strict
-- ToadMan, ToadManSP and ToadManDS: the toadman series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local navi = require("@self/navi")
local navi_chips = require("../lib/navi_chips/chip")
local sp_chips = require("../rules/sp_chips")

-- ToadMan (EXE5 chip 0x101): ToadMan cuts in and plays a chain of paralyzing notes toward the nearest enemy ahead
-- (./navi).
local toadman: Chip = {
    codes = { "T" },
    element = "elec",
    family = "elec",
    class = "mega",
    rarity = 2,
    mb = 53,
    damage = 80,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi_chips.make {
        id = "toadman/controller",
        summon = navi.summon(0),
    },
    auto_battle = { class = "into_reach", range = "ahead_in_row" },
}

-- ToadManSP (EXE5 chip 0x102): ToadMan cuts in and plays a chain of paralyzing notes toward the nearest enemy ahead
-- (../toadman/navi); its damage by how fast its user deleted ToadMan SP (rules/sp_chips: formula 13, its table's row
-- 12).
local toadman_sp: Chip = {
    codes = { "T" },
    element = "elec",
    family = "elec",
    class = "mega",
    rarity = 3,
    mb = 69,
    sp = { by_time = { 220, 210, 200, 190, 180, 170, 160, 150, 140, 130, 120 }, operation_battle = 140 },
    damage = sp_chips.damage,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi_chips.make {
        id = "toadman-sp/controller",
        summon = navi.summon(0),
    },
    auto_battle = { class = "into_reach", range = "ahead_in_row" },
}

-- ToadManDS (EXE5 chip 0x103): ToadMan, dark, cuts in and plays a chain of paralyzing notes toward the nearest enemy
-- ahead (../toadman/navi); its damage by its user's side's statistic 3 (formula 35, its table's row 12).
local toadman_ds: Chip = {
    codes = { "T" },
    element = "elec",
    family = "elec",
    class = "mega",
    rarity = 3,
    mb = 69,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 120, 130, 140, 150, 160, 170, 180, 190, 200, 210, 220 }, operation_battle = 120 },
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi_chips.make {
        id = "toadman-ds/controller",
        summon = navi.summon(3),
    },
    auto_battle = { class = "into_reach", range = "ahead_in_row" },
}

return { toadman = toadman, ["toadman-sp"] = toadman_sp, ["toadman-ds"] = toadman_ds }
