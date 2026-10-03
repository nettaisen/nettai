--!strict
-- OmegaRkt (BN5 chip 0x131): a giga cut-in chip that launches a rocket along the middle row, blowing up what it hits or the enemy's back two columns (./controller).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local controller = require("./controller")

return define.chip {
    id = "bn5:omegarkt",
    codes = { "R" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 89,
    damage = 270,
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage" },
    extra_flags = { 4, 16 },
    slot_in_limit = 1,
    dimming = controller.hook,
}
