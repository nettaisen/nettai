--!strict
-- ElemSwrd: slashes every opposing navi on a grass, ice or volcano panel where it stands.

local parts = require("../../lib/swords/parts")
local strike = require("../../lib/swords/strike")

return define.chip {
    id = "bn6:elemswrd",
    codes = { "J", "M", "U" },
    element = "null", family = "null", class = "standard",
    rarity = 1, mb = 43, damage = 220, hit_param = 30, lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 86, index = 86, sort = 144 },
    slot_in_limit = 3,
    action = strike.action { blade = parts.blades.sword, finds = "on_element_panels", effect_palette = 8 },
}
