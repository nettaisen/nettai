--!strict
-- SpShake1 (BN5 chip 0x018): an electric wave that snakes along the field from the panel ahead (BN5's SpShake,
-- objects/spshake: action 0x51).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local spshake = require("../../objects/spshake/spshake")

return define.chip {
    id = "bn5:spshake1",
    codes = { "G", "H", "I" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 0,
    mb = 30,
    damage = 60,
    hit_param = 20,
    lockout = 20,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = spshake.action("bn5:spshake1/action", spshake.waves[1]),
}
