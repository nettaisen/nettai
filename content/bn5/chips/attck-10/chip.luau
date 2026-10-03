--!strict
-- Attck+10 (BN5 chip 0x095): folded into the chip before it on the custom screen, 10 more damage;
-- used alone, action 0x1A's effect 3 (lib/plus: the sparkle, or in a special battle the side's attack bonus).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local plus = require("../../lib/plus")

return define.chip {
    id = "bn5:attck-10",
    codes = { "*" },
    element = "null",
    family = "plus",
    class = "standard",
    rarity = 0,
    mb = 6,
    damage = 10,
    hit_param = 0,
    lockout = 20,
    flags = { "standard_library", "library" },
    extra_flags = { 64, 128 },
    slot_in_limit = 3,
    modifier = "attack_plus",
    instant = plus.attack,
}
