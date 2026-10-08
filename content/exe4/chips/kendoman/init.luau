--!strict
-- KendoMan, KendMnSP and KendMnDS (EXE4 chips 0x113 to 0x115): navi chips, the dimming chips' action 0x0C, its variant
-- 0x37: each chip's controller (./navi) brings KendoMan, who dashes along his row to the first enemy and slashes it,
-- cuts back from behind and rushes home; each chip states his palette (0x080C8664 by its first parameter, its level
-- 0, 3 or 4: 0, 3, 4). KendMnSP's damage goes by how fast its user deleted KendoMan SP (rules/sp_chips: formula 19, the
-- table's row 18), KendMnDS's by the field's holes (formula 41).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local navi = require("@self/navi")
local navi_chips = require("../lib/navi_chips")
local sp_chips = require("../rules/sp_chips")

-- KendoMan's row of the navi chips' damage (0x08018B48's row 18), by step.
local ROW: { number } = { 130, 125, 120, 115, 110, 105, 100, 95, 90, 85, 80 }

local kendoman: Chip = {
    codes = { "K" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 45,
    damage = 70,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "kendoman/controller", palette = 0 }.hook,
}

local kendmn_sp: Chip = {
    codes = { "K" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 4,
    mb = 75,
    sp = ROW,
    damage = sp_chips.damage,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "kendmn-sp/controller", palette = 3 }.hook,
}

local kendmn_ds: Chip = {
    codes = { "K" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 4,
    mb = 75,
    damage = { formula = "count", of = { panels = "hole" }, by_count = navi_chips.by_holes(ROW) },
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library", "variable_damage" },
    dimming = navi.controller { id = "kendmn-ds/controller", palette = 4 }.hook,
}

return { kendoman = kendoman, ["kendmn-sp"] = kendmn_sp, ["kendmn-ds"] = kendmn_ds }
