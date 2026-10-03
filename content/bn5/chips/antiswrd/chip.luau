--!strict
-- AntiSwrd (BN5 chip 0x092): a trap that answers the next sword with a slash: BN6's (@bn6/lib/traps, its counter
-- the role actions.anti_sword_counter; BN5's the same code).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local traps = require("../../lib/traps")

return define.chip {
    id = "bn5:antiswrd",
    codes = { "D", "O", "R" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 2,
    mb = 43,
    damage = 100,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    trap = "anti_sword",
    dimming = traps.hook(),
}
