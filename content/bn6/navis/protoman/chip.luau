--!strict
-- ProtoMan's own chip, StepSwrd: StepSwrd's step and slash (chips/stepswrd),
-- with ProtoMan's damage by his level.

local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local slash = require("../../lib/swords/slash")

return define.chip {
    id = "bn6:stepswrd-protoman",
    codes = { "B" },
    element = "null",
    family = "sword",
    class = "special",
    rarity = 0,
    mb = 99,
    damage = { formula = "navi_level", base = 60, per_level = 20 },
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library", "variable_damage" },
    library = { number = 410, index = 0, sort = 0 },
    slot_in_limit = 1,
    action = slash.action {
        id = "bn6:stepswrd-protoman/action",
        blade = parts.blades.sword,
        hit = { region = regions.wide, collision = parts.slashes_navis, hit_mod = 3 },
        effect = parts.effects.wide,
        sound = parts.sounds.swing,
        step = true,
    },
}
