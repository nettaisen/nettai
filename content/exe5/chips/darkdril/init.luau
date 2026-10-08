--!strict
-- DarkDril (EXE5 chip 0x0c4): a dark chip: DrilArm's drill arm and two drills on the two panels ahead, for 60
-- ticks, the arm in its second palette and each drill's hit carrying the bug 0xFA (chips/drilarm/drill). Its use costs a
-- panel trail (rules/light_dark).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local drill = require("./drilarm/drill")
local dark_costs = require("../lib/dark_costs")

-- Its cost: the panel trail, at 4.
local DARK_COST = dark_costs.cost(function(stats: Navi)
    stats.panel_trail = 4
    stats.panel_trail_level = 4
end)

local chip: Chip = {
    codes = { "D" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 4,
    mb = 99,
    damage = 100,
    hit_param = 132,
    lockout = 0,
    flags = { "has_damage", "dark" },
    slot_in_limit = 1,
    action = drill.action { ticks = 60, arm_palette = 1, bugs = 0x1FA },
    -- What its use costs its user (rules/light_dark).
    dark_cost = DARK_COST,
    auto_battle = { class = "into_reach", range = "two_in_row" },
}

return { darkdril = chip }
