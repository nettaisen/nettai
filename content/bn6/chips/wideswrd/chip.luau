--!strict
-- WideSwrd: a slash on the column ahead.

local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local slash = require("../../lib/swords/slash")
local slashes = require("../../navis/megaman/forms/slashcross/slashes")
local lockon = require("../../rules/lockon")

return define.chip {
    id = "bn6:wideswrd",
    codes = { "H", "L", "S", "*" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 1,
    mb = 12,
    damage = 80,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.widesht },
    library = { number = 71, index = 71, sort = 509 },
    slot_in_limit = 3,
    action = slash.action {
        id = "bn6:wideswrd/action",
        blade = parts.blades.sword,
        hit = { region = regions.wide, collision = parts.slashes_navis, hit_mod = 3 },
        effect = parts.effects.wide,
        sound = parts.sounds.swing,
        charged = slashes.wide,
    },
}
