--!strict
-- VideoMan, VideMnSP and VideMnDS (EXE4 chips 0x116 to 0x118): navi chips, the dimming chips' action 0x0C, its
-- variant 0x34: each chip's controller (./navi) brings VideoMan, who slams two tapes over squares of panels ahead of
-- him; each chip states his palette (0x080C775C by its first parameter, its level 0, 3 or 4: 0, 5, 6). VideMnSP's
-- damage goes by how fast its user deleted VideoMan SP (rules/sp_chips: formula 20, the table's row 19), VideMnDS's
-- by the field's holes (formula 42). (Their third parameter, 1, is the tapes' course: VideoMan's.)
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local navi = require("@self/navi")
local navi_chips = require("../lib/navi_chips")
local sp_chips = require("../rules/sp_chips")

-- VideoMan's row of the navi chips' damage (0x08018B48's row 19), by step.
local ROW: { number } = { 80, 75, 70, 65, 60, 55, 50, 45, 40, 35, 30 }

local videoman: Chip = {
    codes = { "V" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 44,
    damage = 25,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "videoman/controller", palette = 0 }.hook,
}

local videmn_sp: Chip = {
    codes = { "V" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 4,
    mb = 67,
    sp = ROW,
    damage = sp_chips.damage,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "videmn-sp/controller", palette = 5 }.hook,
}

local videmn_ds: Chip = {
    codes = { "V" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 4,
    mb = 67,
    damage = { formula = "count", of = { panels = "hole" }, by_count = navi_chips.by_holes(ROW) },
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library", "variable_damage" },
    dimming = navi.controller { id = "videmn-ds/controller", palette = 6 }.hook,
}

return { videoman = videoman, ["videmn-sp"] = videmn_sp, ["videmn-ds"] = videmn_ds }
