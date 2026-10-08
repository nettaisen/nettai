--!strict
-- AquaMan, AquaMnSP and AquaMnDS (EXE4 chips 0x0FB to 0x0FD): navi chips, the dimming chips' action 0x0C, its variant
-- 0x07: each chip's controller (./navi) brings AquaMan, who sets his water gun on the panel ahead of him, which sprays
-- the two panels ahead of it; each chip states his palette (0x0800B950 by its first parameter, its level 0, 3 or 4:
-- 0, 2, 3). AquaMnSP's damage goes by how fast its user deleted AquaMan SP (rules/sp_chips: formula 11, the table's
-- row 10), AquaMnDS's by the field's holes (formula 33).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local navi = require("@self/navi")
local navi_chips = require("../lib/navi_chips")
local sp_chips = require("../rules/sp_chips")

-- AquaMan's row of the navi chips' damage (0x08018B48's row 10), by step.
local ROW: { number } = { 180, 170, 160, 150, 140, 130, 120, 110, 100, 90, 80 }

local aquaman: Chip = {
    codes = { "A" },
    element = "aqua",
    family = "aqua",
    class = "mega",
    rarity = 3,
    mb = 41,
    damage = 70,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "aquaman/controller", palette = 0 }.hook,
}

local aquamn_sp: Chip = {
    codes = { "A" },
    element = "aqua",
    family = "aqua",
    class = "mega",
    rarity = 4,
    mb = 63,
    sp = ROW,
    damage = sp_chips.damage,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "aquamn-sp/controller", palette = 2 }.hook,
}

local aquamn_ds: Chip = {
    codes = { "A" },
    element = "aqua",
    family = "aqua",
    class = "mega",
    rarity = 4,
    mb = 63,
    damage = { formula = "count", of = { panels = "hole" }, by_count = navi_chips.by_holes(ROW) },
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library", "variable_damage" },
    dimming = navi.controller { id = "aquamn-ds/controller", palette = 3 }.hook,
}

return { aquaman = aquaman, ["aquamn-sp"] = aquamn_sp, ["aquamn-ds"] = aquamn_ds }
