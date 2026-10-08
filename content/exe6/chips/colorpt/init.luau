--!strict
-- ColorPt, DblPoint: cut-in chips that give the user's front column to the
-- other side, panel by panel, for Atk+10 (ColorPt) or Atk+20 (DblPoint) a
-- panel on the chip at the hand's cursor (controller.luau).

local controller = require("@self/controller")

local chips: { [string]: Chip } = {
    colorpt = {
        codes = { "*" },
        element = "null", family = "plus", class = "standard",
        rarity = 1, mb = 31, damage = 0, hit_param = 0, lockout = 0,
        flags = { "dimming", "standard_library", "library" },
        slot_in_limit = 1,
        dimming = controller.hook { palette = 0, bonus = 10 },
    },
    dblpoint = {
        codes = { "*" },
        element = "null", family = "plus", class = "standard",
        rarity = 3, mb = 50, damage = 0, hit_param = 0, lockout = 0,
        flags = { "dimming", "standard_library", "library" },
        slot_in_limit = 1,
        dimming = controller.hook { palette = 4, bonus = 20 },
    },
}

return chips
