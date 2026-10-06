--!strict
-- AntiDmg: a hidden trap that swallows the next hit of 10 damage or more
-- and answers with its counter (the damage intake's `sub_802CEF4`; the
-- counter is the role `actions.anti_damage_counter`). Its controller is
-- the trap chips' (lib/traps/controller); it sets no object of its own.

local traps = require("../lib/traps/controller")

return define.chip {
    id = "antidmg",
    codes = { "G", "R", "V", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 30,
    damage = 100,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 1,
    trap = "anti_damage",
    dimming = traps.hook(),
}
