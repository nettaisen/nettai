--!strict
-- FireSwrd: a fire slash on the column ahead.

local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local slash = require("../../lib/swords/slash")
local slashes = require("../../navis/00-megaman/forms/slashcross/slashes")
local lockon = require("../../rules/lockon")

return define.chip {
    id = "fireswrd",
    name = "FireSwrd",
    description = "Cut enmy\n3sq fwrd\nw/fire!",
    codes = { "F", "O", "Z" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 1,
    mb = 30,
    damage = 140,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.widesht },
    library = { number = 75, index = 75, sort = 209 },
    slot_in_limit = 3,
    traits = { "element_sword" },
    action = slash.action {
        id = "fireswrd/action",
        blade = parts.blades.fire,
        hit = { region = regions.wide, collision = parts.slashes_navis, hit_mod = 3 },
        effect = parts.effects.wide,
        effect_palette = 1,
        sound = parts.sounds.swing,
        charged = slashes.fire,
    },
}
