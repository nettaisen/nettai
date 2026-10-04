--!strict
-- AntiWood (BN5 chip 0x08f): a trap the other side's first wood hit springs, striking back in wood on every
-- panel the enemy stands on (lib/traps/anti_trap: BN6's ElemTrap trap and counterattack, BN5's).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local anti_trap = require("../lib/traps/anti_trap")

return define.chip {
    id = "antiwood",
    codes = { "M", "S", "Z", "*" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 2,
    mb = 55,
    damage = 200,
    hit_param = 178,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    dimming = anti_trap.hook(4),
}
