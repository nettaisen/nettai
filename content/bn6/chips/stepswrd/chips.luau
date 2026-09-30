--!strict
-- StepSwrd, and ProtoMan's own copy: a step two panels ahead, a slash on the column ahead, and back.

local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local slash = require("../../lib/swords/slash")

return {
    define.chip {
        id = "stepswrd",
        name = "StepSwrd",
        codes = { "B", "L", "P" },
        element = "null", family = "sword", class = "standard",
        rarity = 2, mb = 28, damage = 160, hit_param = 20, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 80, index = 80, sort = 421 },
        slot_in_limit = 3,
        action = slash.action {
            blade = parts.blades.sword,
            hit = { region = regions.wide, collision = parts.slashes_navis, hit_mod = 3 },
            effect = parts.effects.wide,
            sound = parts.sounds.swing,
            step = true,
        },
    },
    define.chip {
        id = "stepswrd-protoman",
        name = "StepSwrd",
        codes = { "B" },
        element = "null", family = "sword", class = "special",
        rarity = 0, mb = 99, damage = { navi_level = { base = 60, per_level = 20 } }, hit_param = 10, lockout = 0,
        flags = { "has_damage", "library", "variable_damage" },
        library = { number = 410, index = 0, sort = 0 },
        slot_in_limit = 1,
        action = slash.action {
            blade = parts.blades.sword,
            hit = { region = regions.wide, collision = parts.slashes_navis, hit_mod = 3 },
            effect = parts.effects.wide,
            sound = parts.sounds.swing,
            step = true,
        },
    },
}
