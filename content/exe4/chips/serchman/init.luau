--!strict
-- SerchMan, SrchMnSP and SrchMnDS (EXE4 chips 0x0E6 to 0x0E8): navi chips, the dimming chips' action 0x0C, its variant
-- 0x20: their controller (./navi) brings SearchMan, whose scope sweeps the enemy area until his user locks it on, and
-- who fires five shots on the panel; each chip states his palette (0x0800B950 by its first parameter, its level 0, 3 or
-- 4: two palettes a variant, times 0, 2 or 3) and his scope's sweep (6 ticks a panel; the SP's and DS's 4). SrchMnSP's
-- damage goes by how fast its user deleted SearchMan SP (rules/sp_chips: formula 4, the table's row 3), SrchMnDS's by
-- the field's holes (formula 26).
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
    dimming = navi.controller { id = "serchman/controller", palette = 0, sweep = 6 }.hook,
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
    dimming = navi.controller { id = "srchmn-sp/controller", palette = 4, sweep = 4 }.hook,
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
    dimming = navi.controller { id = "srchmn-ds/controller", palette = 6, sweep = 4 }.hook,
}

return { serchman = serchman, ["srchmn-sp"] = srchmn_sp, ["srchmn-ds"] = srchmn_ds }
