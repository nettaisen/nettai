--!strict
-- ElecSwrd: an elec slash on the column ahead that paralyzes.

local parts = require("../lib/swords/parts")
local regions = require("@exelib/regions")
local slash = require("../lib/swords/slash")
local slashes = require("../navis/megaman/forms/slashcross/slashes")
local lockon = require("../rules/lockon")
local statuses = require("../rules/status")

return define.chip {
    id = "elecswrd",
    codes = { "E", "K", "N" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 2,
    mb = 35,
    damage = 120,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.widesht },
    slot_in_limit = 3,
    traits = { "element_sword" },
    action = slash.action {
        id = "elecswrd/action",
        blade = parts.blades.elec,
        hit = { region = regions.wide, collision = parts.slashes_navis, hit_mod = 1, status = statuses.paralyze_90 },
        effect = parts.effects.wide,
        effect_palette = 3,
        sound = parts.sounds.swing,
        charged = slashes.elec,
    },
}
