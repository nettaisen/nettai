--!strict
-- AntiSwrd: a hidden trap that swallows the next sword hit and answers
-- with its counter (the damage intake's `sub_802CEF4`; the counter is the
-- role `actions.anti_sword_counter`). Its controller is the trap chips'
-- (lib/traps/controller); it sets no object of its own.

local traps = require("../lib/traps/controller")

return define.chip {
    id = "antiswrd",
    codes = { "A", "R", "Z", "*" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 2,
    mb = 33,
    damage = 100,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 1,
    trap = "anti_sword",
    dimming = traps.hook(),
}
