--!strict
-- LongSwrd: a slash on the two panels ahead.

local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local slash = require("../../lib/swords/slash")

return define.chip {
    id = "longswrd",
    name = "LongSwrd",
    codes = { "H", "L", "S", "*" },
    element = "null", family = "sword", class = "standard",
    rarity = 2, mb = 25, damage = 100, hit_param = 30, lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 72, index = 72, sort = 304 },
    slot_in_limit = 3,
    action = slash.action {
        blade = parts.blades.sword,
        hit = { region = regions.long, collision = parts.slashes_navis, hit_mod = 3 },
        effect = parts.effects.long,
        sound = parts.sounds.swing,
    },
}
