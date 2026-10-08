--!strict
-- WoodMan, WoodMnSP and WoodMnDS (EXE4 chips 0x0FE to 0x100): navi chips, the dimming chips' action 0x0C, its variant
-- 0x3F: each chip's controller (./navi) brings WoodMan, who leaps and, landing, raises wood towers on the other side's
-- panels at random (a second set with his command); each chip states his palette (0x0800B950 by its first parameter,
-- its level 0, 3 or 4: 0, 4, 6). WoodMnSP's damage goes by how fast its user deleted WoodMan SP (rules/sp_chips:
-- formula 12, the table's row 11), WoodMnDS's by the field's holes (formula 34).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local navi = require("@self/navi")
local navi_chips = require("../lib/navi_chips")
local sp_chips = require("../rules/sp_chips")

-- WoodMan's row of the navi chips' damage (0x08018B48's row 11), by step.
local ROW: { number } = { 250, 240, 230, 220, 210, 200, 190, 180, 170, 160, 150 }

local woodman: Chip = {
    codes = { "W" },
    element = "wood",
    family = "wood",
    class = "mega",
    rarity = 3,
    mb = 50,
    damage = 130,
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "woodman/controller", palette = 0 }.hook,
}

local woodmn_sp: Chip = {
    codes = { "W" },
    element = "wood",
    family = "wood",
    class = "mega",
    rarity = 4,
    mb = 76,
    sp = ROW,
    damage = sp_chips.damage,
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "woodmn-sp/controller", palette = 4 }.hook,
}

local woodmn_ds: Chip = {
    codes = { "W" },
    element = "wood",
    family = "wood",
    class = "mega",
    rarity = 4,
    mb = 76,
    damage = { formula = "count", of = { panels = "hole" }, by_count = navi_chips.by_holes(ROW) },
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library", "variable_damage" },
    dimming = navi.controller { id = "woodmn-ds/controller", palette = 6 }.hook,
}

return { woodman = woodman, ["woodmn-sp"] = woodmn_sp, ["woodmn-ds"] = woodmn_ds }
