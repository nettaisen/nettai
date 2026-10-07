--!strict
-- SerchMan, SrchMnSP and SrchMnDS (EXE4 chips 0x0E6 to 0x0E8): navi chips, the dimming chips' action 0x0C, its variant
-- 0x20: their controller (./navi) brings SearchMan, whose scope sweeps the enemy area until his user locks it on, and
-- who fires five shots on the panel; the chip's first parameter is his level (0, 3, 4), which picks his palette and his
-- scope's speed. SrchMnSP's damage goes by how fast its user deleted SearchMan SP (rules/sp_chips: formula 4, the
-- table's row 3), SrchMnDS's by the field's holes (formula 26).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local navi = require("@self/navi")
local navi_chips = require("../lib/navi_chips")
local sp_chips = require("../rules/sp_chips")

-- SearchMan's row of the navi chips' damage (0x08018B48's row 3), by step.
local ROW: { number } = { 75, 70, 65, 60, 55, 50, 45, 40, 35, 30, 25 }

local serchman: Chip = {
    codes = { "S" },
    element = "null",
    family = "invisible",
    class = "mega",
    rarity = 3,
    mb = 45,
    damage = 20,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.hook_for(0),
}

local srchmn_sp: Chip = {
    codes = { "S" },
    element = "null",
    family = "invisible",
    class = "mega",
    rarity = 4,
    mb = 70,
    sp = ROW,
    damage = sp_chips.damage,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.hook_for(3),
}

local srchmn_ds: Chip = {
    codes = { "S" },
    element = "null",
    family = "invisible",
    class = "mega",
    rarity = 4,
    mb = 70,
    damage = { formula = "count", of = { panels = "hole" }, by_count = navi_chips.by_holes(ROW) },
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library", "variable_damage" },
    dimming = navi.hook_for(4),
}

return { serchman = serchman, ["srchmn-sp"] = srchmn_sp, ["srchmn-ds"] = srchmn_ds }
