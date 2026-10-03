--!strict
-- AntiRecv (BN5 chip 0x093): a trap that turns the other side's next recovery into damage: BN6's (@common/traps,
-- its counterattack the role kinds.anti_recovery; BN5's the same code).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local traps = require("../../lib/traps/traps")

return define.chip {
    id = "bn5:antirecv",
    codes = { "B", "J", "P" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 37,
    damage = 0,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 3,
    trap = "anti_recovery",
    dimming = traps.hook(),
}
