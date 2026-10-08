--!strict
-- SerchMan, SerchMnSP and SerchMnDS: the serchman series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local navi = require("@self/navi")
local navi_chips = require("../lib/navi_chips/chip")
local sp_chips = require("../rules/sp_chips")

-- SerchMan (EXE5 chip 0x0e6): SearchMan cuts in, his scope sweeps the enemy area until it locks on, and he fires five
-- shots on the panel (./navi).
local serchman: Chip = {
    codes = { "S" },
    element = "null",
    family = "cursor",
    class = "mega",
    rarity = 2,
    mb = 42,
    damage = 20,
    hit_param = 133,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi_chips.make {
        id = "serchman/controller",
        summon = navi.summon { palette = 0, sweep = 6 },
    },
    auto_battle = { class = "at_once" },
}

-- SerchMnSP (EXE5 chip 0x0e7): SearchMan cuts in, his scope sweeps the enemy area until it locks on, and he fires five
-- shots on the panel (../serchman/navi); its damage by how fast its user deleted SearchMan SP (rules/sp_chips:
-- formula 4, its table's row 3).
local serchmn_sp: Chip = {
    codes = { "S" },
    element = "null",
    family = "cursor",
    class = "mega",
    rarity = 3,
    mb = 72,
    sp = { by_time = { 75, 70, 65, 60, 55, 50, 45, 40, 35, 30, 25 }, operation_battle = 45 },
    damage = sp_chips.damage,
    hit_param = 133,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi_chips.make {
        id = "serchmn-sp/controller",
        summon = navi.summon { palette = 2, sweep = 4 },
    },
    auto_battle = { class = "at_once" },
}

-- SerchMnDS (EXE5 chip 0x0e8): SearchMan, dark, cuts in, his scope sweeps the enemy area until it locks on, and he fires
-- five shots on the panel (../serchman/navi); its damage by its user's side's statistic 3 (formula 26, its table's
-- row 3).
local serchmn_ds: Chip = {
    codes = { "S" },
    element = "null",
    family = "cursor",
    class = "mega",
    rarity = 3,
    mb = 72,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 25, 30, 35, 40, 45, 50, 55, 60, 65, 70, 75 }, operation_battle = 35 },
    hit_param = 133,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi_chips.make {
        id = "serchmn-ds/controller",
        summon = navi.summon { palette = 3, sweep = 4 },
    },
    auto_battle = { class = "at_once" },
}

return { serchman = serchman, ["serchmn-sp"] = serchmn_sp, ["serchmn-ds"] = serchmn_ds }
