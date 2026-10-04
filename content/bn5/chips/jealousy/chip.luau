--!strict
-- Jealousy (BN5 chip 0x0d4): a cut-in chip whose effect hits every panel the opponent's side stands on once for
-- each chip the opponent holds, 10 ticks apart (./controller: BN5's effect object #0x36, BN6's code that no BN6 chip
-- uses).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local controller = require("./controller")

return define.chip {
    id = "bn5:jealousy",
    codes = { "J" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 35,
    damage = 80,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    extra_flags = { 128 },
    slot_in_limit = 1,
    dimming = controller.hook,
}
