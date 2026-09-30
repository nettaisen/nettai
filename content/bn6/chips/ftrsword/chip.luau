--!strict
-- FtrSword: a slash on the three panels ahead.

local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local slash = require("../../lib/swords/slash")

return define.chip {
    id = "ftrsword",
    name = "FtrSword",
    codes = { "E", "L", "S", "*" },
    element = "null", family = "sword", class = "special",
    rarity = 0, mb = 8, damage = 80, hit_param = 30, lockout = 0,
    flags = { "has_damage", "library" },
    library = { number = 370, index = 0, sort = 0 },
    slot_in_limit = 1,
    -- The Beast rush, and its lock-on mode (a number until rules/lockon.luau).
    beast = { lockon = 5 },
    action = slash.action {
        blade = parts.blades.sword,
        hit = { region = regions.line, collision = parts.slashes_navis, hit_mod = 3 },
        effect = parts.effects.line,
        sound = parts.sounds.swing,
    },
}
