--!strict
-- ColorPt, DblPoint: cut-in chips that give the user's front column to the
-- other side, panel by panel, for Atk+10 (ColorPt) or Atk+20 (DblPoint) a
-- panel on the chip at the hand's cursor (controller.luau).

local controller = require("@self/controller")

return {
    define.chip {
        id = "colorpt",
        codes = { "*" },
        element = "null", family = "plus", class = "standard",
        rarity = 1, mb = 31, damage = 0, hit_param = 0, lockout = 0,
        flags = { "dimming", "standard_library", "library" },
        library = { number = 199, index = 199, sort = 108 },
        slot_in_limit = 1,
        dimming = controller.hook { palette = 0, bonus = 10 },
    },
    define.chip {
        id = "dblpoint",
        codes = { "*" },
        element = "null", family = "plus", class = "standard",
        rarity = 3, mb = 50, damage = 0, hit_param = 0, lockout = 0,
        flags = { "dimming", "standard_library", "library" },
        library = { number = 200, index = 200, sort = 119 },
        slot_in_limit = 1,
        dimming = controller.hook { palette = 1, bonus = 20 },
    },
}
