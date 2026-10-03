--!strict
-- Geddon2 (BN5 chip 0x07c): a quake breaks every solid panel of the field (cracking those something stands on), one
-- every 8 ticks in a random order: BN6's Geddon without holding the user's panel (objects/geddon).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local controller = require("../../objects/geddon/controller")

return define.chip {
    id = "bn5:geddon2",
    codes = { "H", "T", "W" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 47,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 1,
    dimming = controller.hook("break"),
}
