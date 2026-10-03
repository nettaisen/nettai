--!strict
-- Barr200 (BN5 chip 0x089): a barrier of 200 HP on the user: BN6's barrier chips (the dimming handler's subtype 14)
-- with BN5's barriers (lib/barriers).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local barriers = require("../../lib/barriers")

return define.chip {
    id = "bn5:barr200",
    codes = { "W" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 47,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 1,
    megaman = "light",
    dimming = barriers.controller.hook(barriers.barrier_200),
}
