--!strict
-- Otenko: a statue in front of the user that, for 1800 ticks, raises the
-- Atk+ bonus of the user's next damaging chip by 1 every 5 ticks, up to 50
-- (docs/engine/dimming-chip-effects.md §9). Its record is the Japanese
-- games' (the content's JP-content chips take theirs): the US games' but
-- for the library flag and the sort key.

local statue = require("./statue")

return define.chip {
    id = "bn6:otenko",
    codes = { "O" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 4,
    mb = 66,
    damage = 1,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library" },
    extra_flags = { 1 },
    library = { number = 202, index = 2, sort = 51 },
    slot_in_limit = 1,
    dimming = statue.hook,
}
