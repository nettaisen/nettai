--!strict
-- ThunMan, ThunMnSP and ThunMnDS (EXE4 chips 0x0EC to 0x0EE): navi chips, the dimming chips' action 0x0C, its variant
-- 0x23: the shared navi chip controller (objects/navi_chip_controller, the chips' third parameter 4) brings ThunMan
-- (./navi), who calls a thunderbolt on each row of the column three ahead of him; the chip's first parameter is his
-- level (0, 3, 4), which picks his palette. ThunMnSP's damage goes by how fast its user deleted ThunMan SP
-- (rules/sp_chips: formula 6, the table's row 5), ThunMnDS's by the field's holes (formula 28).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local controller = require("../objects/navi_chip_controller")
local navi_chips = require("../lib/navi_chips")
local sp_chips = require("../rules/sp_chips")

-- ThunMan's row of the navi chips' damage (0x08018B48's row 5), by step.
local ROW: { number } = { 220, 210, 200, 190, 180, 170, 160, 150, 140, 130, 120 }
-- The shared controller's ThunMan (the chips' third parameter).
local THUNMAN = 4

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
    dimming = controller.hook_for(0, THUNMAN),
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
    dimming = controller.hook_for(3, THUNMAN),
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
    dimming = controller.hook_for(4, THUNMAN),
}

return { thunman = thunman, ["thunmn-sp"] = thunmn_sp, ["thunmn-ds"] = thunmn_ds }
