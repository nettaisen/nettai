--!strict
-- LaserMan, LasrMnSP and LasrMnDS (EXE4 chips 0x110 to 0x112): navi chips, the dimming chips' action 0x0C, its variant
-- 0x43: each chip's controller (./navi) brings LaserMan, who fires his laser along his row: LaserMan's hits; LasrMnSP's
-- and LasrMnDS's aimed by the direction their user holds while dimmed, its course stripping NaviCust programs before it
-- hits. Each chip states his palette (0x080CAC54 by its first parameter, its level 0, 3 or 4: 0, 10, 11) and whether
-- he aims. LasrMnSP's damage goes by how fast its user deleted LaserMan SP (rules/sp_chips: formula 18, the table's
-- row 17), LasrMnDS's by the field's holes (formula 40).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local navi = require("@self/navi")
local navi_chips = require("../lib/navi_chips")
local sp_chips = require("../rules/sp_chips")

-- LaserMan's row of the navi chips' damage (0x08018B48's row 17), by step.
local ROW: { number } = { 220, 210, 200, 190, 180, 170, 160, 150, 140, 130, 120 }

local laserman: Chip = {
    codes = { "L" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 60,
    damage = 100,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "laserman/controller", palette = 0, aims = false }.hook,
}

local lasrmn_sp: Chip = {
    codes = { "L" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 4,
    mb = 80,
    sp = ROW,
    damage = sp_chips.damage,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "lasrmn-sp/controller", palette = 10, aims = true }.hook,
}

local lasrmn_ds: Chip = {
    codes = { "L" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 4,
    mb = 80,
    damage = { formula = "count", of = { panels = "hole" }, by_count = navi_chips.by_holes(ROW) },
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library", "variable_damage" },
    dimming = navi.controller { id = "lasrmn-ds/controller", palette = 11, aims = true }.hook,
}

return { laserman = laserman, ["lasrmn-sp"] = lasrmn_sp, ["lasrmn-ds"] = lasrmn_ds }
