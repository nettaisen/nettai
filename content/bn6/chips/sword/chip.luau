--!strict
-- Sword: a slash on the panel ahead.

local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local slash = require("../../lib/swords/slash")

return define.chip {
    id = "sword",
    name = "Sword",
    codes = { "H", "L", "S", "*" },
    element = "null", family = "sword", class = "standard",
    rarity = 0, mb = 8, damage = 80, hit_param = 30, lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 70, index = 70, sort = 426 },
    slot_in_limit = 3,
    action = slash.action {
        blade = parts.blades.sword,
        hit = { region = regions.single, collision = parts.slashes_navis, hit_mod = 3 },
        effect = parts.effects.slash,
        sound = parts.sounds.swing,
    },
}
