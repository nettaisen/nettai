--!strict
-- BugFix: a cut-in chip that fixes the user's NaviCust bugs while a copy
-- of its navi glows over it (controller.luau).

local bugfix = require("@self/controller")

local chip: Chip = {
    codes = { "K", "P", "Z", "*" },
    element = "null", family = "null", class = "standard",
    rarity = 3, mb = 62, damage = 0, hit_param = 0, lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 1,
    dimming = bugfix.spawn,
}

return { bugfix = chip }
