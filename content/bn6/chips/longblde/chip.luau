--!strict
-- LongBlde: LongSwrd's slash, stronger.

local parts = require("../../lib/swords/parts")
local regions = require("@common/regions")
local slash = require("../../lib/swords/slash")
local slashes = require("../../navis/megaman/forms/slashcross/slashes")
local lockon = require("../../rules/lockon")

return define.chip {
    id = "bn6:longblde",
    codes = { "B", "M", "V" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 3,
    mb = 38,
    damage = 150,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.firebrn1 },
    library = { number = 74, index = 74, sort = 303 },
    slot_in_limit = 3,
    action = slash.action {
        id = "bn6:longblde/action",
        blade = parts.blades.sword,
        hit = { region = regions.long, collision = parts.slashes_navis, hit_mod = 3 },
        effect = parts.effects.long_blade,
        sound = parts.sounds.swing,
        charged = slashes.long_blade,
    },
}
