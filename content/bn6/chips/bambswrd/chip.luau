--!strict
-- BambSwrd: a wood slash on the column ahead.

local parts = require("../../lib/swords/parts")
local regions = require("@exelib/regions")
local slash = require("../../lib/swords/slash")
local slashes = require("../../navis/megaman/forms/slashcross/slashes")
local lockon = require("../../rules/lockon")

return define.chip {
    id = "bn6:bambswrd",
    codes = { "H", "S", "W" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 2,
    mb = 34,
    damage = 150,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.widesht },
    library = { number = 78, index = 78, sort = 26 },
    slot_in_limit = 3,
    traits = { "element_sword" },
    action = slash.action {
        id = "bn6:bambswrd/action",
        blade = parts.blades.bamboo,
        hit = { region = regions.wide, collision = parts.slashes_navis, hit_mod = 1 },
        effect = parts.effects.wide,
        effect_palette = 4,
        sound = parts.sounds.swing,
        charged = slashes.bamboo,
    },
}
