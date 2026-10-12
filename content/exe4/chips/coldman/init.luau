--!strict
-- ColdMan, ColdMnSP and ColdMnDS (EXE4 chips 0x10A to 0x10C): navi chips, the dimming chips' action 0x0C, its variant
-- 0x49: each chip's controller (./navi) brings ColdMan, who encases the field's obstacles (those he can stand behind
-- in bubbles that become ice cubes, the others in ice) and punches each cube toward his enemy, then forms one ahead of
-- him and punches it. Each chip states his palette (0x080CC0BE by its first parameter, its level 0, 3 or 4: 0, 2, 3).
-- ColdMnSP's damage goes by how fast its user deleted ColdMan SP (rules/sp_chips: formula 16, the table's row 15),
-- ColdMnDS's by the field's holes (formula 38).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local navi = require("@self/navi")
local navi_chips = require("../lib/navi_chips")
local sp_chips = require("../rules/sp_chips")

-- ColdMan's row of the navi chips' damage (0x08018B48's row 15), by step.
local ROW: { number } = { 230, 220, 210, 200, 190, 180, 170, 160, 150, 140, 130 }

local coldman: Chip = {
    codes = { "C" },
    element = "aqua",
    family = "aqua",
    class = "mega",
    rarity = 3,
    mb = 53,
    damage = 100,
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "coldman/controller", palette = 0 }.hook,
}

local coldmn_sp: Chip = {
    codes = { "C" },
    element = "aqua",
    family = "aqua",
    class = "mega",
    rarity = 4,
    mb = 72,
    sp = ROW,
    damage = sp_chips.damage,
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "coldmn-sp/controller", palette = 2 }.hook,
}

local coldmn_ds: Chip = {
    codes = { "C" },
    element = "aqua",
    family = "aqua",
    class = "mega",
    rarity = 4,
    mb = 72,
    damage = { formula = "count", of = { panels = "hole" }, by_count = navi_chips.by_holes(ROW) },
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library", "variable_damage" },
    dimming = navi.controller { id = "coldmn-ds/controller", palette = 3 }.hook,
}

return { coldman = coldman, ["coldmn-sp"] = coldmn_sp, ["coldmn-ds"] = coldmn_ds }
