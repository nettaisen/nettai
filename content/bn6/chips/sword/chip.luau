--!strict
-- Sword: a slash on the panel ahead.

local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local slash = require("../../lib/swords/slash")
local slashes = require("../../navis/megaman/forms/slashcross/slashes")
local lockon = require("../../rules/lockon")

return define.chip {
    id = "sword",
    codes = { "H", "L", "S", "*" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 0,
    mb = 8,
    damage = 80,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.thunder },
    library = { number = 70, index = 70, sort = 426 },
    slot_in_limit = 3,
    action = slash.action {
        id = "sword/action",
        blade = parts.blades.sword,
        hit = { region = regions.single, collision = parts.slashes_navis, hit_mod = 3 },
        effect = parts.effects.slash,
        sound = parts.sounds.swing,
        charged = slashes.sword,
    },
}
