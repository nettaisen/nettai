--!strict
-- Muramasa: a slash on the two panels ahead, as strong as the HP its user lost.

local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local slash = require("../../lib/swords/slash")

return define.chip {
    id = "muramasa",
    name = "Muramasa",
    codes = { "M" },
    element = "null", family = "sword", class = "standard",
    rarity = 3, mb = 77, damage = { damage_taken = {} }, hit_param = 50, lockout = 0,
    flags = { "has_damage", "standard_library", "damage_shown_variable", "library", "variable_damage" },
    library = { number = 84, index = 84, sort = 320 },
    slot_in_limit = 1,
    action = slash.action {
        blade = parts.blades.sword,
        hit = { region = regions.long, collision = parts.slashes_navis, hit_mod = 3 },
        effect = parts.effects.muramasa,
        sound = parts.sounds.swing,
    },
}
