--!strict
-- AntiDmg (BN5 chip 0x091): a hidden trap that swallows the next hit of 10 damage or more and counters with
-- shurikens: BN6's AntiDmg (@exelib/traps/controller, the dimming spawner's entry 12; BN5's the same code; its
-- counter the role `actions.anti_damage_counter`).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local traps = require("../lib/traps")

return define.chip {
    id = "antidmg",
    codes = { "C", "F", "M", "*" },
    element = "null",
    family = "invisible",
    class = "standard",
    rarity = 2,
    mb = 31,
    damage = 120,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    trap = "anti_damage",
    dimming = traps.hook(),
}
