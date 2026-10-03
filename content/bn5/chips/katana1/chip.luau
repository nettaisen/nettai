--!strict
-- Katana1 (BN5 chip 0x03c): two slashes, a wide one and then a long one (BN5's Katana, objects/katana: action 0x54,
-- its own action on the swords' slash; a charged one steps first).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local katana = require("../../objects/katana/katana")

return define.chip {
    id = "bn5:katana1",
    codes = { "D", "H", "S" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 0,
    mb = 25,
    damage = 60,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = katana.action,
}
