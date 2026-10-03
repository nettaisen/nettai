--!strict
-- CusVolt3 (BN5 chip 0x0a8): an electric beam over the panel and the column ahead that follows the navi for 25 ticks
-- (objects/cusvolt, action 0x27), draining half the custom gauge; its damage 50 plus up to 100 by how full the gauge is
-- (BN5's formula 75: none for a full one).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local cusvolt = require("../../objects/cusvolt/cusvolt")

return define.chip {
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
    slot_in_limit = 3,
    damage = { formula = "gauge_level", base = 50 },
    action = cusvolt.action("bn5:cusvolt3/action"),
}
