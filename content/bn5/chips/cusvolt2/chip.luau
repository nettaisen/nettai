--!strict
-- CusVolt2 (BN5 chip 0x0a7): an electric beam over the panel and the column ahead that follows the navi for 25 ticks
-- (objects/cusvolt, action 0x27), draining half the custom gauge; its damage 30 plus up to 100 by how full the gauge is
-- (BN5's formula 74: none for a full one).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local cusvolt = require("../../objects/cusvolt/cusvolt")

return define.chip {
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
    slot_in_limit = 3,
    damage = { formula = "gauge_level", base = 30 },
    action = cusvolt.action("bn5:cusvolt2/action"),
}
