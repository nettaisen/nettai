--!strict
-- Roll, RollSP and RollDS: the roll series (each chip's own comment above its definition).
-- Their records are written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("@self/navi")

-- Roll (BN5 chip 0x0dd): Roll whips the enemy navi ahead and heals her user with a heart (./navi).
local roll = define.chip {
    id = "roll",
    codes = { "R" },
    element = "null",
    family = "recovery",
    class = "mega",
    rarity = 2,
    mb = 20,
    damage = 20,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    -- (AntiRecv springs on her heal; she brings her user back herself.)
    traits = { "heals", "navi_returns_user" },
    navi = navi.summon { rounds = 1, palette = 0, level = 0 },
}

-- RollSP (BN5 chip 0x0de): Roll (../roll/navi), turning her side's holy panels Normal as she comes; its damage by
-- the holy panels of its user's side (formula 1, 0x0800E8B4).
local roll_sp = define.chip {
    id = "roll-sp",
    codes = { "R" },
    element = "null",
    family = "recovery",
    class = "mega",
    rarity = 3,
    mb = 60,
    damage = { formula = "count", of = { own_panels = "holy" }, by_count = { 30, 35, 40, 45, 50, 55, 60, 65, 70, 75, 80 }, operation_battle = 50 },
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    slot_in_limit = 1,
    megaman = "light",
    -- (AntiRecv springs on her heal; she brings her user back herself.)
    traits = { "heals", "navi_returns_user" },
    navi = navi.summon { rounds = 1, palette = 0, level = 3 },
}

-- RollDS (BN5 chip 0x0df): Roll (../roll/navi) in her dark palette; its damage by its user's side's statistic 3
-- (formula 23, its table's row 0).
local roll_ds = define.chip {
    id = "roll-ds",
    codes = { "R" },
    element = "null",
    family = "recovery",
    class = "mega",
    rarity = 3,
    mb = 60,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 30, 35, 40, 45, 50, 55, 60, 65, 70, 75, 80 }, operation_battle = 40 },
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    slot_in_limit = 1,
    megaman = "dark",
    -- (AntiRecv springs on her heal; she brings her user back herself.)
    traits = { "heals", "navi_returns_user" },
    navi = navi.summon { rounds = 1, palette = 1, level = 4, ribbon_palette = 1 },
}

return { roll = roll, roll_sp = roll_sp, roll_ds = roll_ds }
