--!strict
-- SparkMan, SprkMnSP and SprkMnDS (EXE4 chips 0x10D to 0x10F): navi chips, the dimming chips' action 0x0C, its variant
-- 0x03: each chip's controller (./navi) brings SparkMan, who blinds the other side's navi with a flash and sends his
-- sparks zigzagging over the field from the panel ahead of him; each chip states his palette (0x080B8A30 by its first
-- parameter, its level 0, 3 or 4: 0, 3, 6). SprkMnSP's damage goes by how fast its user deleted SparkMan SP
-- (rules/sp_chips: formula 17, the table's row 16), SprkMnDS's by the field's holes (formula 39).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local navi = require("@self/navi")
local navi_chips = require("../lib/navi_chips")
local sp_chips = require("../rules/sp_chips")

-- SparkMan's row of the navi chips' damage (0x08018B48's row 16), by step.
local ROW: { number } = { 200, 190, 180, 170, 160, 150, 140, 130, 120, 110, 100 }

local sparkman: Chip = {
    codes = { "S" },
    element = "elec",
    family = "elec",
    class = "mega",
    rarity = 3,
    mb = 55,
    damage = 70,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "sparkman/controller", palette = 0 }.hook,
}

local sprkmn_sp: Chip = {
    codes = { "S" },
    element = "elec",
    family = "elec",
    class = "mega",
    rarity = 4,
    mb = 77,
    sp = ROW,
    damage = sp_chips.damage,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "sprkmn-sp/controller", palette = 3 }.hook,
}

local sprkmn_ds: Chip = {
    codes = { "S" },
    element = "elec",
    family = "elec",
    class = "mega",
    rarity = 4,
    mb = 77,
    damage = { formula = "count", of = { panels = "hole" }, by_count = navi_chips.by_holes(ROW) },
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library", "variable_damage" },
    dimming = navi.controller { id = "sprkmn-ds/controller", palette = 6 }.hook,
}

return { sparkman = sparkman, ["sprkmn-sp"] = sprkmn_sp, ["sprkmn-ds"] = sprkmn_ds }
