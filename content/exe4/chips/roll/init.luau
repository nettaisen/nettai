--!strict
-- Roll, RollSP and RollDS (EXE4 chips 0x0DD to 0x0DF): navi chips, the dimming chips' action 0x0C, its variant 0x14
-- (0x080E5516): Roll's controller (./navi) brings Roll, who whips the enemy navi ahead and heals her side's navi with a
-- heart; the chip's first parameter is her level (0, 3, 4), which picks her palette and what her heart adds to the
-- mood. Unless the battle is dimmed, the other side's armed AntiRecv springs instead (lib/navi_chips' `anti_recovery`).
-- RollSP's damage goes by how fast its user deleted Roll SP (rules/sp_chips: formula 1, the table's row 0), RollDS's by
-- the field's holes (formula 23, 0x08019518: the row's entry 10 less the holes, ten at most).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local navi = require("@self/navi")
local navi_chips = require("../lib/navi_chips")
local sp_chips = require("../rules/sp_chips")

-- Roll's row of the navi chips' damage (0x08018B48's row 0), by step.
local ROW: { number } = { 80, 75, 70, 65, 60, 55, 50, 45, 40, 35, 30 }

local roll: Chip = {
    codes = { "R" },
    element = "null",
    family = "recovery",
    class = "mega",
    rarity = 3,
    mb = 28,
    damage = 20,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi_chips.anti_recovery(navi.hook_for(0)),
}

local roll_sp: Chip = {
    codes = { "R" },
    element = "null",
    family = "recovery",
    class = "mega",
    rarity = 4,
    mb = 60,
    sp = ROW,
    damage = sp_chips.damage,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi_chips.anti_recovery(navi.hook_for(3)),
}

local roll_ds: Chip = {
    codes = { "R" },
    element = "null",
    family = "recovery",
    class = "mega",
    rarity = 4,
    mb = 60,
    damage = { formula = "count", of = { panels = "hole" }, by_count = navi_chips.by_holes(ROW) },
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library", "variable_damage" },
    dimming = navi_chips.anti_recovery(navi.hook_for(4)),
}

return { roll = roll, ["roll-sp"] = roll_sp, ["roll-ds"] = roll_ds }
