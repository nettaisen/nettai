--!strict
-- MetalMan, MetlMnSP and MetlMnDS (EXE4 chips 0x0F5 to 0x0F7): navi chips, the dimming chips' action 0x0C, its variant
-- 0x3C: their controller (./navi) brings MetalMan, whose user aims his punch while the battle is dimmed; each chip
-- states his palette (0x0800B950 by its first parameter, its level 0, 3 or 4: two palettes a variant, times 0, 2 or 3).
-- MetlMnSP's damage goes by how fast its user deleted MetalMan SP (rules/sp_chips: formula 9, the table's row 8),
-- MetlMnDS's by the field's holes (formula 31).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local navi = require("@self/navi")
local navi_chips = require("../lib/navi_chips")
local sp_chips = require("../rules/sp_chips")

-- MetalMan's row of the navi chips' damage (0x08018B48's row 8), by step.
local ROW: { number } = { 300, 290, 280, 270, 260, 250, 240, 230, 220, 210, 200 }

local metalman: Chip = {
    codes = { "M" },
    element = "null",
    family = "metal",
    class = "mega",
    rarity = 3,
    mb = 52,
    damage = 160,
    hit_param = 178,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "metalman/controller", palette = 0 }.hook,
}

local metlmn_sp: Chip = {
    codes = { "M" },
    element = "null",
    family = "metal",
    class = "mega",
    rarity = 4,
    mb = 74,
    sp = ROW,
    damage = sp_chips.damage,
    hit_param = 178,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "metlmn-sp/controller", palette = 4 }.hook,
}

local metlmn_ds: Chip = {
    codes = { "M" },
    element = "null",
    family = "metal",
    class = "mega",
    rarity = 4,
    mb = 74,
    damage = { formula = "count", of = { panels = "hole" }, by_count = navi_chips.by_holes(ROW) },
    hit_param = 178,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library", "variable_damage" },
    dimming = navi.controller { id = "metlmn-ds/controller", palette = 6 }.hook,
}

return { metalman = metalman, ["metlmn-sp"] = metlmn_sp, ["metlmn-ds"] = metlmn_ds }
