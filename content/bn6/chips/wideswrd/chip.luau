--!strict
-- WideSwrd: a slash on the column ahead.

local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local slash = require("../../lib/swords/slash")

return define.chip {
    id = "wideswrd",
    name = "WideSwrd",
    codes = { "H", "L", "S", "*" },
    element = "null", family = "sword", class = "standard",
    rarity = 1, mb = 12, damage = 80, hit_param = 30, lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 71, index = 71, sort = 509 },
    slot_in_limit = 3,
    action = slash.action {
        blade = parts.blades.sword,
        hit = { region = regions.wide, collision = parts.slashes_navis, hit_mod = 3 },
        effect = parts.effects.wide,
        sound = parts.sounds.swing,
    },
}
