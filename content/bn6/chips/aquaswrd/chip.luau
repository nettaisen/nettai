--!strict
-- AquaSwrd: an aqua slash on the column ahead.

local parts = require("../../lib/swords/parts")
local regions = require("@common/regions")
local slash = require("../../lib/swords/slash")
local slashes = require("../../navis/megaman/forms/slashcross/slashes")
local lockon = require("../../rules/lockon")

return define.chip {
    id = "bn6:aquaswrd",
    codes = { "A", "I", "Y" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 2,
    mb = 32,
    damage = 160,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.widesht },
    library = { number = 76, index = 76, sort = 18 },
    slot_in_limit = 3,
    traits = { "element_sword" },
    action = slash.action {
        id = "bn6:aquaswrd/action",
        blade = parts.blades.aqua,
        hit = { region = regions.wide, collision = parts.slashes_navis, hit_mod = 3 },
        effect = parts.effects.wide,
        effect_palette = 2,
        sound = parts.sounds.swing,
        charged = slashes.aqua,
    },
}
