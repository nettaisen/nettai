--!strict
-- FireMan, FireMnSP and FireMnDS (EXE4 chips 0x0E9 to 0x0EB): navi chips, the dimming chips' action 0x0C, its variant
-- 0x04: each chip's controller (./navi) brings FireMan, who breathes fire along his row from the panel ahead (his
-- command makes it leave lava); each chip states his palette (0x0800B950 by its first parameter, its level 0, 3 or 4:
-- 0, 2, 3). FireMnSP's damage goes by how fast its user deleted FireMan SP (rules/sp_chips: formula 5, the table's row
-- 4), FireMnDS's by the field's holes (formula 27).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local navi = require("@self/navi")
local navi_chips = require("../lib/navi_chips")
local sp_chips = require("../rules/sp_chips")

-- FireMan's row of the navi chips' damage (0x08018B48's row 4), by step.
local ROW: { number } = { 250, 240, 230, 220, 210, 200, 190, 180, 170, 160, 150 }

local fireman: Chip = {
    codes = { "F" },
    element = "fire",
    family = "fire",
    class = "mega",
    rarity = 3,
    mb = 36,
    damage = 100,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "fireman/controller", palette = 0 }.hook,
}

local firemn_sp: Chip = {
    codes = { "F" },
    element = "fire",
    family = "fire",
    class = "mega",
    rarity = 4,
    mb = 72,
    sp = ROW,
    damage = sp_chips.damage,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "firemn-sp/controller", palette = 2 }.hook,
}

local firemn_ds: Chip = {
    codes = { "F" },
    element = "fire",
    family = "fire",
    class = "mega",
    rarity = 4,
    mb = 72,
    damage = { formula = "count", of = { panels = "hole" }, by_count = navi_chips.by_holes(ROW) },
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library", "variable_damage" },
    dimming = navi.controller { id = "firemn-ds/controller", palette = 3 }.hook,
}

return { fireman = fireman, ["firemn-sp"] = firemn_sp, ["firemn-ds"] = firemn_ds }
