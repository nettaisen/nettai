--!strict
-- WoodNos1 (BN5 chip 0x0b2): the navi sends a wooden wave along its row; past the field it raises geysers under the
-- other side's navis: BN5's own (objects/woodnos).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local woodnos = require("../../objects/woodnos/woodnos")

return define.chip {
    id = "bn5:woodnos1",
    codes = { "F", "R", "W" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 0,
    mb = 27,
    damage = 100,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = woodnos.action,
}
