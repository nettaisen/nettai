--!strict
-- AntiWatr (BN5 chip 0x08d): a trap the other side's first aqua hit springs, striking back in aqua on every
-- panel the enemy stands on (lib/traps/anti_trap: BN6's ElemTrap trap and counterattack, BN5's).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local anti_trap = require("../../lib/traps/anti_trap")

return define.chip {
    id = "antiwatr",
    codes = { "D", "G", "Q", "*" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 2,
    mb = 54,
    damage = 200,
    hit_param = 178,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    dimming = anti_trap.hook(2),
}
