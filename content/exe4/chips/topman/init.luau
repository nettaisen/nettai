--!strict
-- TopMan, TopMnSP and TopMnDS (EXE4 chips 0x101 to 0x103): navi chips, the dimming chips' action 0x0C, its variant
-- 0x01: each chip's controller (./navi) brings TopMan, who sets four tops spinning off the panels ahead and leaps over
-- the field to land with three breaking hits; each chip states his palette (0x080B8590 by its first parameter, its
-- level 0, 3 or 4: 0, 3, 6). TopMnSP's damage goes by how fast its user deleted TopMan SP (rules/sp_chips: formula 13,
-- the table's row 12), TopMnDS's by the field's holes (formula 35).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local navi = require("@self/navi")
local navi_chips = require("../lib/navi_chips")
local sp_chips = require("../rules/sp_chips")

-- TopMan's row of the navi chips' damage (0x08018B48's row 12), by step.
local ROW: { number } = { 80, 75, 70, 65, 60, 55, 50, 45, 40, 35, 30 }

local topman: Chip = {
    codes = { "T" },
    element = "null",
    family = "metal",
    class = "mega",
    rarity = 3,
    mb = 54,
    damage = 20,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "topman/controller", palette = 0 }.hook,
}

local topmn_sp: Chip = {
    codes = { "T" },
    element = "null",
    family = "metal",
    class = "mega",
    rarity = 4,
    mb = 71,
    sp = ROW,
    damage = sp_chips.damage,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "topmn-sp/controller", palette = 3 }.hook,
}

local topmn_ds: Chip = {
    codes = { "T" },
    element = "null",
    family = "metal",
    class = "mega",
    rarity = 4,
    mb = 71,
    damage = { formula = "count", of = { panels = "hole" }, by_count = navi_chips.by_holes(ROW) },
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library", "variable_damage" },
    dimming = navi.controller { id = "topmn-ds/controller", palette = 6 }.hook,
}

return { topman = topman, ["topmn-sp"] = topmn_sp, ["topmn-ds"] = topmn_ds }
