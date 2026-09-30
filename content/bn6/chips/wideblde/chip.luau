--!strict
-- WideBlde: WideSwrd's slash, stronger.

local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local slash = require("../../lib/swords/slash")

return define.chip {
    id = "wideblde",
    name = "WideBlde",
    codes = { "B", "R", "W" },
    element = "null", family = "sword", class = "standard",
    rarity = 3, mb = 38, damage = 150, hit_param = 30, lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 73, index = 73, sort = 507 },
    slot_in_limit = 3,
    action = slash.action {
        blade = parts.blades.sword,
        hit = { region = regions.wide, collision = parts.slashes_navis, hit_mod = 3 },
        effect = parts.effects.wide_blade,
        sound = parts.sounds.swing,
    },
}
