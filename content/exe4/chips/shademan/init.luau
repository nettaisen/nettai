--!strict
-- ShadeMan, ShadMnSP and ShadMnDS (EXE4 chips 0x104 to 0x106): navi chips, the dimming chips' action 0x0C, its variant
-- 0x06: each chip's controller (./navi) brings ShadeMan, who crushes the panel ahead and the column beyond (a command
-- paralyzes or confuses; without one the crush brings a move bug); each chip states his palette (0x080BADE0 by its
-- first parameter, its level 0, 3 or 4: 0, 6, 14). ShadMnSP's damage goes by how fast its user deleted ShadeMan SP
-- (rules/sp_chips: formula 14, the table's row 13), ShadMnDS's by the field's holes (formula 36).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local navi = require("@self/navi")
local navi_chips = require("../lib/navi_chips")
local sp_chips = require("../rules/sp_chips")

-- ShadeMan's row of the navi chips' damage (0x08018B48's row 13), by step.
local ROW: { number } = { 240, 230, 220, 210, 200, 190, 180, 170, 160, 150, 140 }

local shademan: Chip = {
    codes = { "X" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 73,
    damage = 120,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "shademan/controller", palette = 0 }.hook,
}

local shadmn_sp: Chip = {
    codes = { "X" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 4,
    mb = 88,
    sp = ROW,
    damage = sp_chips.damage,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "shadmn-sp/controller", palette = 6 }.hook,
}

local shadmn_ds: Chip = {
    codes = { "X" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 4,
    mb = 88,
    damage = { formula = "count", of = { panels = "hole" }, by_count = navi_chips.by_holes(ROW) },
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library", "variable_damage" },
    dimming = navi.controller { id = "shadmn-ds/controller", palette = 14 }.hook,
}

return { shademan = shademan, ["shadmn-sp"] = shadmn_sp, ["shadmn-ds"] = shadmn_ds }
