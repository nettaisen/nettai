--!strict
-- StepSwrd: a step two panels ahead, a slash on the column ahead, and back. (ProtoMan's own copy is
-- protoman.luau.)

local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local slash = require("../../lib/swords/slash")
local slashes = require("../../navis/00-megaman/forms/slashcross/slashes")

return define.chip {
    id = "stepswrd",
    name = "StepSwrd",
    codes = { "B", "L", "P" },
    element = "null", family = "sword", class = "standard",
    rarity = 2, mb = 28, damage = 160, hit_param = 20, lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 80, index = 80, sort = 421 },
    slot_in_limit = 3,
    -- The Beast rush, and its lock-on mode (a number until rules/lockon.luau).
    beast = { lockon = 16 },
    action = slash.action {
        blade = parts.blades.sword,
        hit = { region = regions.wide, collision = parts.slashes_navis, hit_mod = 3 },
        effect = parts.effects.wide,
        sound = parts.sounds.swing,
        step = true,
        charged = slashes.wide,
    },
}
