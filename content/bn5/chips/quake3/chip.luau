--!strict
-- Quake3 (BN5 chip 0x025): a bomb lobbed three panels ahead that drops a weight there and, landing on solid ground,
-- cracks the panel and hits around it (level 2): BN5's throw (lib/bombs, action 0x12 subtype 11) and its Quake bomb
-- (objects/quake, attack object #0x51).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local bombs = require("../../lib/bombs")
local quake = require("../../objects/quake/quake")

return define.chip {
    id = "bn5:quake3",
    codes = { "F", "H", "N" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 2,
    mb = 40,
    damage = 200,
    hit_param = 50,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = bombs.throw { held = bombs.held, thrower = quake.thrower(2) },
}
