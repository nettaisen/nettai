--!strict
-- Geddon1 (BN5 chip 0x07b): a quake cracks every solid panel of the field, one every 8 ticks in a random order:
-- BN6's Geddon (its quake's first row) without holding the user's panel (objects/geddon).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local controller = require("../../objects/geddon/controller")

return define.chip {
    id = "bn5:geddon1",
    codes = { "M", "P", "Z", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 34,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 1,
    dimming = controller.hook("crack"),
}
