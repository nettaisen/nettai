--!strict
-- AreaGrab: a dimming that drops grab shots on the first column in front
-- not wholly the user's side's, taking the panels it can.

local grab = require("../lib/grab/controller")

local chip: Chip = {
    codes = { "B", "F", "S", "*" },
    element = "null", family = "null", class = "standard",
    rarity = 1, mb = 8, damage = 10, hit_param = 138, lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 1,
    dimming = grab.area,
}

return { areagrab = chip }
