--!strict
-- AntiRecv: a hidden trap that turns the other side's next recovery into
-- damage (`sub_800E2FC` and Roll's `sub_80E192C`, with the counterattack
-- in ./controller). The controller that sets it is the trap chips'
-- (lib/traps/controller); it sets no object of its own.

local traps = require("../../lib/traps/controller")

return define.chip {
    id = "bn6:antirecv",
    codes = { "A", "F", "V", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 37,
    damage = 0,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    library = { number = 191, index = 191, sort = 11 },
    slot_in_limit = 1,
    trap = "anti_recovery",
    dimming = traps.hook(),
}
