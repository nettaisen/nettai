--!strict
-- AssnSwrd: slashes every opposing navi paralyzed or on a grass, ice or volcano panel
-- where it stands.

local parts = require("../lib/swords/parts")
local strike = require("../lib/swords/strike")

return define.chip {
    id = "assnswrd",
    codes = { "N", "R", "Y" },
    element = "null", family = "null", class = "standard",
    rarity = 2, mb = 50, damage = 240, hit_param = 30, lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 87, index = 87, sort = 20 },
    slot_in_limit = 3,
    action = strike.action { blade = parts.blades.sword, finds = "either", effect_palette = 9 },
}
