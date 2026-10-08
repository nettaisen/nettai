--!strict
-- GutsMan, GutsMnSP and GutsMnDS (EXE4 chips 0x0E0 to 0x0E2): navi chips, the dimming chips' action 0x0C, its variant
-- 0x05: each chip's controller (./navi) brings GutsMan, who strikes the panel ahead with his hammer and sets off his
-- quake there, cracking the panels ahead (six at random; the SP's and the DS's all) and dropping rocks on the other
-- side; each chip states his palette (0x0800B950 by its first parameter, its level 0, 3 or 4: 0, 2, 3) and his
-- quake's cracks. GutsMnSP's damage goes by how fast its user deleted GutsMan SP (rules/sp_chips: formula 2, the
-- table's row 1), GutsMnDS's by the field's holes (formula 24).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local navi = require("@self/navi")
local navi_chips = require("../lib/navi_chips")
local sp_chips = require("../rules/sp_chips")

-- GutsMan's row of the navi chips' damage (0x08018B48's row 1), by step.
local ROW: { number } = { 200, 190, 180, 170, 160, 150, 140, 130, 120, 110, 100 }

local gutsman: Chip = {
    codes = { "G" },
    element = "null",
    family = "break",
    class = "mega",
    rarity = 3,
    mb = 32,
    damage = 80,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "gutsman/controller", palette = 0, cracks = "six" }.hook,
}

local gutsmn_sp: Chip = {
    codes = { "G" },
    element = "null",
    family = "break",
    class = "mega",
    rarity = 4,
    mb = 68,
    sp = ROW,
    damage = sp_chips.damage,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "gutsmn-sp/controller", palette = 2, cracks = "all" }.hook,
}

local gutsmn_ds: Chip = {
    codes = { "G" },
    element = "null",
    family = "break",
    class = "mega",
    rarity = 4,
    mb = 68,
    damage = { formula = "count", of = { panels = "hole" }, by_count = navi_chips.by_holes(ROW) },
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library", "variable_damage" },
    dimming = navi.controller { id = "gutsmn-ds/controller", palette = 3, cracks = "all" }.hook,
}

return { gutsman = gutsman, ["gutsmn-sp"] = gutsmn_sp, ["gutsmn-ds"] = gutsmn_ds }
