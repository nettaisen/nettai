--!strict
-- AquaSwrd: an aqua slash on the column ahead.

local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local slash = require("../../lib/swords/slash")

return define.chip {
    id = "aquaswrd",
    name = "AquaSwrd",
    codes = { "A", "I", "Y" },
    element = "aqua", family = "aqua", class = "standard",
    rarity = 2, mb = 32, damage = 160, hit_param = 30, lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 76, index = 76, sort = 18 },
    slot_in_limit = 3,
    action = slash.action {
        blade = parts.blades.aqua,
        hit = { region = regions.wide, collision = parts.slashes_navis, hit_mod = 3 },
        effect = parts.effects.wide,
        effect_palette = 2,
        sound = parts.sounds.swing,
    },
}
