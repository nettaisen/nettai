--!strict
-- ProtoMan, ProtoMnSP and ProtoMnDS (EXE4 chips 0x0EF to 0x0F1): navi chips, the dimming chips' action 0x0C, its
-- variant 0x2F: their controller (./navi) brings ProtoMan, who cuts in and slashes each enemy navi he can reach; each
-- chip states his palette (0x0800B950 by its first parameter, its level 0, 3 or 4: two palettes a variant, times 0, 2
-- or 3). ProtoMnSP's damage goes by how fast its user deleted ProtoMan SP (rules/sp_chips: formula 7, the table's row
-- 6), ProtoMnDS's by the field's holes (formula 29).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local navi = require("@self/navi")
local navi_chips = require("../lib/navi_chips")
local sp_chips = require("../rules/sp_chips")

-- ProtoMan's row of the navi chips' damage (0x08018B48's row 6), by step.
local ROW: { number } = { 240, 230, 220, 210, 200, 190, 180, 170, 160, 150, 140 }

local protoman: Chip = {
    codes = { "B" },
    element = "null",
    family = "sword",
    class = "mega",
    rarity = 3,
    mb = 54,
    damage = 100,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "protoman/controller", palette = 0 }.hook,
}

local protom_sp: Chip = {
    codes = { "B" },
    element = "null",
    family = "sword",
    class = "mega",
    rarity = 4,
    mb = 79,
    sp = ROW,
    damage = sp_chips.damage,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "protom-sp/controller", palette = 4 }.hook,
}

local protom_ds: Chip = {
    codes = { "B" },
    element = "null",
    family = "sword",
    class = "mega",
    rarity = 4,
    mb = 79,
    damage = { formula = "count", of = { panels = "hole" }, by_count = navi_chips.by_holes(ROW) },
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library", "variable_damage" },
    dimming = navi.controller { id = "protom-ds/controller", palette = 6 }.hook,
}

return { protoman = protoman, ["protom-sp"] = protom_sp, ["protom-ds"] = protom_ds }
