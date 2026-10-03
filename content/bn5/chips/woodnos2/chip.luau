--!strict
-- WoodNos2 (BN5 chip 0x0b3): the navi sends a wooden wave along its row; past the field it raises geysers under the
-- other side's navis: BN5's own (objects/woodnos).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local woodnos = require("../../objects/woodnos/woodnos")

return define.chip {
    id = "bn5:woodnos2",
    codes = { "J", "T", "Z" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 1,
    mb = 40,
    damage = 130,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = woodnos.action,
}
