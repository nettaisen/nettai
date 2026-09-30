--!strict
-- BambSwrd: a wood slash on the column ahead.

local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local slash = require("../../lib/swords/slash")

return define.chip {
    id = "bambswrd",
    name = "BambSwrd",
    codes = { "H", "S", "W" },
    element = "wood", family = "wood", class = "standard",
    rarity = 2, mb = 34, damage = 150, hit_param = 30, lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 78, index = 78, sort = 26 },
    slot_in_limit = 3,
    action = slash.action {
        blade = parts.blades.bamboo,
        hit = { region = regions.wide, collision = parts.slashes_navis, hit_mod = 1 },
        effect = parts.effects.wide,
        effect_palette = 4,
        sound = parts.sounds.swing,
    },
}
