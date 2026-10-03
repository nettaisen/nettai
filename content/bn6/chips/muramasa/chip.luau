--!strict
-- Muramasa: a slash on the two panels ahead, as strong as the HP its user lost.

local parts = require("../../lib/swords/parts")
local regions = require("@common/regions")
local slash = require("../../lib/swords/slash")
local slashes = require("../../navis/megaman/forms/slashcross/slashes")
local lockon = require("../../rules/lockon")

return define.chip {
    id = "bn6:muramasa",
    codes = { "M" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 3,
    mb = 77,
    damage = { formula = "hp_lost" },
    hit_param = 50,
    lockout = 0,
    flags = { "has_damage", "standard_library", "damage_shown_variable", "library", "variable_damage" },
    beast = { lockon = lockon.firebrn1 },
    library = { number = 84, index = 84, sort = 320 },
    slot_in_limit = 1,
    action = slash.action {
        id = "bn6:muramasa/action",
        blade = parts.blades.sword,
        hit = { region = regions.long, collision = parts.slashes_navis, hit_mod = 3 },
        effect = parts.effects.muramasa,
        sound = parts.sounds.swing,
        charged = slashes.muramasa,
    },
}
