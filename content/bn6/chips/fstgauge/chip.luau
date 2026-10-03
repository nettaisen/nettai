--!strict
-- FstGauge: the custom gauge fills fast for the rest of the round
-- (lib/gauge-speed/controller).

local gauge_speed = require("../../lib/gauge-speed/controller")

return define.chip {
    id = "bn6:fstgauge",
    codes = { "E", "M", "R", "*" },
    element = "null", family = "null", class = "standard",
    rarity = 2, mb = 48, damage = 0, hit_param = 0, lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    library = { number = 175, index = 175, sort = 213 },
    slot_in_limit = 1,
    dimming = gauge_speed.fast,
}
