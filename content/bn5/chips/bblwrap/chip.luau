--!strict
-- BblWrap (BN5 chip 0x086): a bubble on the user that takes one hit, grows back 240 ticks later and breaks on elec:
-- BN6's barrier chips (the dimming handler's subtype 14) with BN5's barriers (lib/barriers).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local barriers = require("../../lib/barriers")

return define.chip {
    id = "bblwrap",
    codes = { "Y" },
    element = "null",
    family = "aqua",
    class = "standard",
    rarity = 2,
    mb = 35,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 1,
    dimming = barriers.controller.hook(barriers.bubble),
}
