--!strict
-- MchnSwrd: slashes every paralyzed opposing navi where it stands.

local parts = require("../../lib/swords/parts")
local strike = require("../../lib/swords/strike")

return define.chip {
    id = "mchnswrd",
    name = "MchnSwrd",
    codes = { "H", "L", "Q" },
    element = "null", family = "null", class = "standard",
    rarity = 0, mb = 37, damage = 200, hit_param = 30, lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 85, index = 85, sort = 312 },
    slot_in_limit = 3,
    action = strike.action { blade = parts.blades.sword, finds = "paralyzed", effect_palette = 7 },
}
