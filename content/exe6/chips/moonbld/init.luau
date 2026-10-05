--!strict
-- MoonBld: a spin with a blade that sweeps the eight panels around the
-- navi (./action, ./blade).

local action = require("@self/action")
local slashes = require("../navis/megaman/forms/slashcross/slashes")
local lockon = require("../rules/lockon")

return define.chip {
    id = "moonbld",
    codes = { "A", "M", "T" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 2,
    mb = 35,
    damage = 130,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.moonbld },
    library = { number = 83, index = 83, sort = 318 },
    slot_in_limit = 3,
    -- (SlashCross's A-charge sends its ring as a sword wave.)
    action = action.action { id = "moonbld/action", charged = slashes.moon },
}
