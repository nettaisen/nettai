--!strict
-- WindMan, WindMnSP and WindMnDS (EXE4 chips 0x0E3 to 0x0E5): navi chips, the dimming chips' action 0x0C, its variant
-- 0x21: each chip's controller (./navi) brings WindMan, who blows three tornadoes from the panel ahead of him, each
-- going down a row, two panels ahead, up to the field's edge and back toward his side; each chip states his palette
-- (0x0800B950 by its first parameter, its level 0, 3 or 4: 0, 4, 6). WindMnSP's damage goes by how fast its user
-- deleted WindMan SP (rules/sp_chips: formula 3, the table's row 2), WindMnDS's by the field's holes (formula 25).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local navi = require("@self/navi")
local navi_chips = require("../lib/navi_chips")
local sp_chips = require("../rules/sp_chips")

-- WindMan's row of the navi chips' damage (0x08018B48's row 2), by step.
local ROW: { number } = { 100, 95, 90, 85, 80, 75, 70, 65, 60, 55, 50 }

local windman: Chip = {
    codes = { "W" },
    element = "null",
    family = "wind",
    class = "mega",
    rarity = 3,
    mb = 48,
    damage = 40,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "windman/controller", palette = 0 }.hook,
}

local windmn_sp: Chip = {
    codes = { "W" },
    element = "null",
    family = "wind",
    class = "mega",
    rarity = 4,
    mb = 64,
    sp = ROW,
    damage = sp_chips.damage,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "windmn-sp/controller", palette = 4 }.hook,
}

local windmn_ds: Chip = {
    codes = { "W" },
    element = "null",
    family = "wind",
    class = "mega",
    rarity = 4,
    mb = 64,
    damage = { formula = "count", of = { panels = "hole" }, by_count = navi_chips.by_holes(ROW) },
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library", "variable_damage" },
    dimming = navi.controller { id = "windmn-ds/controller", palette = 6 }.hook,
}

return { windman = windman, ["windmn-sp"] = windmn_sp, ["windmn-ds"] = windmn_ds }
