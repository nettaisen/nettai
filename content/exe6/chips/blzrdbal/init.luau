--!strict
-- BlzrdBal: the user's stand-in throws a snowball that rolls to the field's end,
-- hitting once a column and growing with each field object it swallows
-- (docs/engine/dimming-chip-effects.md §10).

local controller = require("@self/controller")

return define.chip {
    id = "blzrdbal",
    codes = { "H", "N", "T" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 3,
    mb = 39,
    damage = 150,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    library = { number = 115, index = 115, sort = 43 },
    slot_in_limit = 1,
    dimming = controller.hook,
}
