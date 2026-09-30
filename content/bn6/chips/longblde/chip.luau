--!strict
-- LongBlde: LongSwrd's slash, stronger.

local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local slash = require("../../lib/swords/slash")

return define.chip {
    id = "longblde",
    name = "LongBlde",
    codes = { "B", "M", "V" },
    element = "null", family = "sword", class = "standard",
    rarity = 3, mb = 38, damage = 150, hit_param = 30, lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 74, index = 74, sort = 303 },
    slot_in_limit = 3,
    action = slash.action {
        blade = parts.blades.sword,
        hit = { region = regions.long, collision = parts.slashes_navis, hit_mod = 3 },
        effect = parts.effects.long_blade,
        sound = parts.sounds.swing,
    },
}
