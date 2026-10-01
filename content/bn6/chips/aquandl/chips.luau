--!strict
-- AquaNdl1-3: a volley of three aqua needles dropped on the enemies'
-- panels (./action), each level's needles in its own colours.
--
-- AquaNdl3's record stays the pack's for now (MstrCros's Program Advance
-- recipe names it by number): this module gives its action, which
-- registration by number runs (chips/03f-aquandl3; docs/design/
-- content-model-v2.md §12).

local action = require("./action")

return {
    define.chip {
        id = "aquandl1",
        name = "AquaNdl1",
        description = "Target 3\nneedles\nat enemy!",
        codes = { "C", "J", "P" },
        element = "aqua", family = "aqua", class = "standard",
        rarity = 0, mb = 31, damage = 40, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 112, index = 112, sort = 15 },
        slot_in_limit = 3,
        action = action.action { palette = 0 },
    },
    define.chip {
        id = "aquandl2",
        name = "AquaNdl2",
        description = "Target 3\nneedles\nat enemy!",
        codes = { "F", "K", "T" },
        element = "aqua", family = "aqua", class = "standard",
        rarity = 1, mb = 35, damage = 60, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 113, index = 113, sort = 16 },
        slot_in_limit = 3,
        action = action.action { palette = 1 },
    },
    -- AquaNdl3's record's action.
    { action = action.action { id = "aquandl3/action", palette = 2 } },
}
