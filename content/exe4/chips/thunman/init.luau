--!strict
-- ThunMan, ThunMnSP and ThunMnDS (EXE4 chips 0x0EC to 0x0EE): navi chips, the dimming chips' action 0x0C, its variant
-- 0x23: their controller (./navi) brings ThunMan, who calls a thunderbolt on each row of the column three ahead of him;
-- each chip states his palette (0x0800B950 by its first parameter, its level 0, 3 or 4: two palettes a variant, times
-- 0, 2 or 3). ThunMnSP's damage goes by how fast its user deleted ThunMan SP (rules/sp_chips: formula 6, the table's
-- row 5), ThunMnDS's by the field's holes (formula 28).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local navi = require("@self/navi")
local navi_chips = require("../lib/navi_chips")
local sp_chips = require("../rules/sp_chips")

-- ThunMan's row of the navi chips' damage (0x08018B48's row 5), by step.
local ROW: { number } = { 220, 210, 200, 190, 180, 170, 160, 150, 140, 130, 120 }

local thunman: Chip = {
    codes = { "T" },
    element = "elec",
    family = "elec",
    class = "mega",
    rarity = 3,
    mb = 48,
    damage = 90,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "thunman/controller", palette = 0 }.hook,
}

local thunmn_sp: Chip = {
    codes = { "T" },
    element = "elec",
    family = "elec",
    class = "mega",
    rarity = 4,
    mb = 68,
    sp = ROW,
    damage = sp_chips.damage,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "thunmn-sp/controller", palette = 4 }.hook,
}

local thunmn_ds: Chip = {
    codes = { "T" },
    element = "elec",
    family = "elec",
    class = "mega",
    rarity = 4,
    mb = 68,
    damage = { formula = "count", of = { panels = "hole" }, by_count = navi_chips.by_holes(ROW) },
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library", "variable_damage" },
    dimming = navi.controller { id = "thunmn-ds/controller", palette = 6 }.hook,
}

return { thunman = thunman, ["thunmn-sp"] = thunmn_sp, ["thunmn-ds"] = thunmn_ds }
