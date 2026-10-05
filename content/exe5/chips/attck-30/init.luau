--!strict
-- Attck+30 (EXE5 chip 0x0cd): folded into the chip before it on the custom screen, 30 more damage;
-- used alone, action 0x1A's effect 3 (lib/plus: the sparkle, or in a special battle the side's attack bonus).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local plus = require("../lib/plus")

return define.chip {
    id = "attck-30",
    codes = { "*" },
    element = "null",
    family = "plus",
    class = "mega",
    rarity = 3,
    mb = 66,
    damage = 30,
    hit_param = 0,
    lockout = 20,
    flags = { "library" },
    extra_flags = { 64, 128 },
    slot_in_limit = 1,
    modifier = "attack_plus",
    instant = plus.attack,
}
