--!strict
-- BurnMan, BurnMnSP and BurnMnDS (EXE4 chips 0x107 to 0x109): navi chips, the dimming chips' action 0x0C, its variant
-- 0x08: each chip's controller (./navi) brings BurnMan, who raises fire pillars two columns ahead, a row above and
-- below, and breathes his flame over the three panels ahead; each chip states his palette (0x080BE1D8 by its first
-- parameter, its level 0, 3 or 4: 0, 2, 4). BurnMnSP's damage goes by how fast its user deleted BurnMan SP
-- (rules/sp_chips: formula 15, the table's row 14), BurnMnDS's by the field's holes (formula 37).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local navi = require("@self/navi")
local navi_chips = require("../lib/navi_chips")
local sp_chips = require("../rules/sp_chips")

-- BurnMan's row of the navi chips' damage (0x08018B48's row 14), by step.
local ROW: { number } = { 120, 115, 110, 105, 100, 95, 90, 85, 80, 75, 70 }

local burnman: Chip = {
    codes = { "B" },
    element = "fire",
    family = "fire",
    class = "mega",
    rarity = 3,
    mb = 46,
    damage = 60,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "burnman/controller", palette = 0 }.hook,
}

local burnmn_sp: Chip = {
    codes = { "B" },
    element = "fire",
    family = "fire",
    class = "mega",
    rarity = 4,
    mb = 69,
    sp = ROW,
    damage = sp_chips.damage,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "burnmn-sp/controller", palette = 2 }.hook,
}

local burnmn_ds: Chip = {
    codes = { "B" },
    element = "fire",
    family = "fire",
    class = "mega",
    rarity = 4,
    mb = 69,
    damage = { formula = "count", of = { panels = "hole" }, by_count = navi_chips.by_holes(ROW) },
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library", "variable_damage" },
    dimming = navi.controller { id = "burnmn-ds/controller", palette = 4 }.hook,
}

return { burnman = burnman, ["burnmn-sp"] = burnmn_sp, ["burnmn-ds"] = burnmn_ds }
