--!strict
-- SloGauge: the custom gauge fills slowly for the rest of the round
-- (lib/gauge_speed/controller).

local gauge_speed = require("../lib/gauge_speed/controller")

return define.chip {
    id = "slogauge",
    codes = { "A", "B", "G", "*" },
    element = "null", family = "null", class = "standard",
    rarity = 2, mb = 42, damage = 0, hit_param = 0, lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    library = { number = 174, index = 174, sort = 411 },
    slot_in_limit = 1,
    dimming = gauge_speed.slow,
}
