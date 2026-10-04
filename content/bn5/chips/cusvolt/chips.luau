--!strict
-- CusVolt1, CusVolt2 and CusVolt3: the cusvolt series (each chip's own comment above its definition).
-- Their records are written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local cusvolt = require("./cusvolt")

-- CusVolt1 (BN5 chip 0x0a6): an electric beam over the panel and the column ahead that follows the navi for 25 ticks
-- (chips/cusvolt/cusvolt, action 0x27), draining half the custom gauge; its damage 10 plus up to 100 by how full the gauge is
-- (BN5's formula 73: none for a full one).
local cusvolt1 = define.chip {
    id = "bn5:cusvolt1",
    codes = { "A", "V", "Y", "*" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 1,
    mb = 40,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "damage_shown_variable", "library", "variable_damage" },
    -- (The custom screen shows its damage as "???" on an A, and the damage on another code.)
    traits = { "hides_damage_as_a" },
    slot_in_limit = 3,
    damage = { formula = "gauge_level", base = 10 },
    action = cusvolt.action("bn5:cusvolt1/action"),
}

-- CusVolt2 (BN5 chip 0x0a7): an electric beam over the panel and the column ahead that follows the navi for 25 ticks
-- (chips/cusvolt/cusvolt, action 0x27), draining half the custom gauge; its damage 30 plus up to 100 by how full the gauge is
-- (BN5's formula 74: none for a full one).
local cusvolt2 = define.chip {
    id = "bn5:cusvolt2",
    codes = { "C", "E", "G" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 2,
    mb = 55,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "damage_shown_variable", "library", "variable_damage" },
    -- (The custom screen shows its damage as "???" only on an A, which it never comes as.)
    traits = { "hides_damage_as_a" },
    slot_in_limit = 3,
    damage = { formula = "gauge_level", base = 30 },
    action = cusvolt.action("bn5:cusvolt2/action"),
}

-- CusVolt3 (BN5 chip 0x0a8): an electric beam over the panel and the column ahead that follows the navi for 25 ticks
-- (chips/cusvolt/cusvolt, action 0x27), draining half the custom gauge; its damage 50 plus up to 100 by how full the gauge is
-- (BN5's formula 75: none for a full one).
local cusvolt3 = define.chip {
    id = "bn5:cusvolt3",
    codes = { "G", "M", "P" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 3,
    mb = 70,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "damage_shown_variable", "library", "variable_damage" },
    -- (The custom screen shows its damage as "???" only on an A, which it never comes as.)
    traits = { "hides_damage_as_a" },
    slot_in_limit = 3,
    damage = { formula = "gauge_level", base = 50 },
    action = cusvolt.action("bn5:cusvolt3/action"),
}

return { cusvolt1 = cusvolt1, cusvolt2 = cusvolt2, cusvolt3 = cusvolt3 }
