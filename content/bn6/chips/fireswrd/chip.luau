--!strict
-- FireSwrd: a fire slash on the column ahead.

local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local slash = require("../../lib/swords/slash")

return define.chip {
    id = "fireswrd",
    name = "FireSwrd",
    codes = { "F", "O", "Z" },
    element = "fire", family = "fire", class = "standard",
    rarity = 1, mb = 30, damage = 140, hit_param = 30, lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 75, index = 75, sort = 209 },
    slot_in_limit = 3,
    action = slash.action {
        blade = parts.blades.fire,
        hit = { region = regions.wide, collision = parts.slashes_navis, hit_mod = 3 },
        effect = parts.effects.wide,
        effect_palette = 1,
        sound = parts.sounds.swing,
    },
}
