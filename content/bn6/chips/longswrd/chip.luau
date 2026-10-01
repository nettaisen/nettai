--!strict
-- LongSwrd: a slash on the two panels ahead.

local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local slash = require("../../lib/swords/slash")
local slashes = require("../../navis/00-megaman/forms/slashcross/slashes")
local lockon = require("../../rules/lockon")

return define.chip {
    id = "longswrd",
    name = "LongSwrd",
    description = "Cuts enmy\nin front!\nRange: 2",
    codes = { "H", "L", "S", "*" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 2,
    mb = 25,
    damage = 100,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.firebrn1 },
    library = { number = 72, index = 72, sort = 304 },
    slot_in_limit = 3,
    action = slash.action {
        id = "longswrd/action",
        blade = parts.blades.sword,
        hit = { region = regions.long, collision = parts.slashes_navis, hit_mod = 3 },
        effect = parts.effects.long,
        sound = parts.sounds.swing,
        charged = slashes.long,
    },
}
