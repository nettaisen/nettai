--!strict
-- AntiNavi: a hidden trap that turns the other side's next navi chip
-- around, so its navi comes for this side instead (the dimming service's
-- `sub_800BDB2`). Its controller is the trap chips' (lib/traps/controller);
-- it sets no object of its own.

local traps = require("../lib/traps/controller")

return define.chip {
    id = "antinavi",
    codes = { "F", "L", "T", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 50,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 1,
    trap = "anti_navi",
    dimming = traps.hook(),
}
