--!strict
-- Geddon3 (BN5 chip 0x07d): a quake turns every solid panel of the field to poison, one every 8 ticks in a random
-- order: BN6's Geddon (its quake's third row, which no BN6 chip uses) without holding the user's panel
-- (objects/geddon).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local controller = require("../../objects/geddon/controller")

return define.chip {
    id = "bn5:geddon3",
    codes = { "A", "O", "V" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 62,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 1,
    dimming = controller.hook("poison"),
}
