--!strict
-- PanlGrab: a dimming that drops a grab shot on the first panel past the
-- user's side's front in its row, taking the panel if it can.

local grab = require("../../lib/grab/controller")

return define.chip {
    id = "panlgrab",
    codes = { "*" },
    element = "null", family = "null", class = "standard",
    rarity = 0, mb = 6, damage = 10, hit_param = 138, lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    library = { number = 164, index = 164, sort = 325 },
    slot_in_limit = 1,
    dimming = grab.panel,
}
