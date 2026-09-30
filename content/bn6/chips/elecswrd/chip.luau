--!strict
-- ElecSwrd: an elec slash on the column ahead that paralyzes.

local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local slash = require("../../lib/swords/slash")

return define.chip {
    id = "elecswrd",
    name = "ElecSwrd",
    codes = { "E", "K", "N" },
    element = "elec", family = "elec", class = "standard",
    rarity = 2, mb = 35, damage = 120, hit_param = 30, lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 77, index = 77, sort = 143 },
    slot_in_limit = 3,
    action = slash.action {
        blade = parts.blades.elec,
        hit = { region = regions.wide, collision = parts.slashes_navis, hit_mod = 1, status = 0x10 },
        effect = parts.effects.wide,
        effect_palette = 3,
        sound = parts.sounds.swing,
    },
}
