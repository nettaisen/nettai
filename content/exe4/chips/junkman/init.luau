--!strict
-- JunkMan, JunkMnSP and JunkMnDS (EXE4 chips 0x0F8 to 0x0FA): navi chips, the dimming chips' action 0x0C, its variant
-- 0x40: the shared navi chip controller (objects/navi_chip_controller, the chips' third parameter 0x14) brings JunkMan
-- (./navi), who throws every obstacle on the field at the nearest of the other side's navis in front of him (the
-- poltergeist, @exelib/poltergeist); the chip's first parameter is his level (0, 3, 4), which picks his palette.
-- JunkMnSP's damage goes by how fast its user deleted JunkMan SP (rules/sp_chips: formula 10, the table's row 9),
-- JunkMnDS's by the field's holes (formula 32).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local controller = require("../objects/navi_chip_controller")
local navi_chips = require("../lib/navi_chips")
local sp_chips = require("../rules/sp_chips")

-- JunkMan's row of the navi chips' damage (0x08018B48's row 9), by step.
local ROW: { number } = { 200, 190, 180, 170, 160, 150, 140, 130, 120, 110, 100 }
-- The shared controller's JunkMan (the chips' third parameter).
local JUNKMAN = 0x14

local junkman: Chip = {
    codes = { "J" },
    element = "null",
    family = "summon",
    class = "mega",
    rarity = 3,
    mb = 80,
    damage = 100,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = controller.hook_for(0, JUNKMAN),
}

local junkmn_sp: Chip = {
    codes = { "J" },
    element = "null",
    family = "summon",
    class = "mega",
    rarity = 4,
    mb = 80,
    sp = ROW,
    damage = sp_chips.damage,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = controller.hook_for(3, JUNKMAN),
}

local junkmn_ds: Chip = {
    codes = { "J" },
    element = "null",
    family = "summon",
    class = "mega",
    rarity = 4,
    mb = 80,
    damage = { formula = "count", of = { panels = "hole" }, by_count = navi_chips.by_holes(ROW) },
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library", "variable_damage" },
    dimming = controller.hook_for(4, JUNKMAN),
}

return { junkman = junkman, ["junkmn-sp"] = junkmn_sp, ["junkmn-ds"] = junkmn_ds }
