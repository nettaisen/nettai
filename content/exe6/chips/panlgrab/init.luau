--!strict
-- PanlGrab: a dimming that drops a grab shot on the first panel past the
-- user's side's front in its row, taking the panel if it can.

local grab = require("../lib/grab/controller")

local chip: Chip = {
    codes = { "*" },
    element = "null", family = "null", class = "standard",
    rarity = 0, mb = 6, damage = 10, hit_param = 138, lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 1,
    dimming = grab.panel,
}

return { panlgrab = chip }
