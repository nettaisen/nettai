--!strict
-- NumbrBl: a cut-in chip whose NumberMan sets four balls rolling down the
-- user's row; its damage is the last two digits of the user's HP
-- (docs/engine/dimming-chip-effects.md §15).

local controller = require("./controller")

return define.chip {
    id = "bn6:numbrbl",
    codes = { "N" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 4,
    mb = 69,
    damage = { formula = "hp_last_digits" },
    hit_param = 138,
    lockout = 0,
    flags = {
        "dimming",
        "has_damage",
        "standard_library",
        "damage_shown_variable",
        "library",
        "variable_damage",
    },
    library = { number = 104, index = 104, sort = 323 },
    slot_in_limit = 1,
    dimming = controller.hook(4),
}
