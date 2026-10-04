--!strict
-- DarkWide (BN5 chip 0x0c0): a dark chip: WideSht's shot (@exelib/widesht) with BN5's shooter and a wave of kind 9
-- (chips/widesht: it goes through and trails), one wave (its subtype's count, 0x080ED008). Its use costs buster
-- blanks (rules/light-dark).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local wave = require("../widesht/variants")
local widesht = require("../widesht/wave")

return define.chip {
    id = "darkwide",
    codes = { "T" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 4,
    mb = 99,
    damage = 300,
    hit_param = 158,
    lockout = 0,
    flags = { "has_damage", "dark" },
    slot_in_limit = 1,
    action = widesht.action { id = "darkwide/action", wave = wave.dark, waves = 1, z = 0 },
}
