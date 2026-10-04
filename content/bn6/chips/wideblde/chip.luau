--!strict
-- WideBlde: WideSwrd's slash, stronger.

local parts = require("../../lib/swords/parts")
local regions = require("@exelib/regions")
local slash = require("../../lib/swords/slash")
local slashes = require("../../navis/megaman/forms/slashcross/slashes")
local lockon = require("../../rules/lockon")

return define.chip {
    id = "bn6:wideblde",
    codes = { "B", "R", "W" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 3,
    mb = 38,
    damage = 150,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.widesht },
    library = { number = 73, index = 73, sort = 507 },
    slot_in_limit = 3,
    action = slash.action {
        id = "bn6:wideblde/action",
        blade = parts.blades.sword,
        hit = { region = regions.wide, collision = parts.slashes_navis, hit_mod = 3 },
        effect = parts.effects.wide_blade,
        sound = parts.sounds.swing,
        charged = slashes.wide_blade,
    },
}
