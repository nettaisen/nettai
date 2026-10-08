--!strict
-- ProtoMan's own chip, StepSwrd: StepSwrd's step and slash (chips/stepswrd),
-- with ProtoMan's damage by his level.

local parts = require("../../lib/swords/parts")
local regions = require("@exelib/regions")
local slash = require("../../lib/swords/slash")

-- The slash over a column.
local WIDE_SLASH = new.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 0 }

local chip: Chip = {
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
    slot_in_limit = 1,
    action = slash.action {
        id = "stepswrd-protoman/action",
        blade = parts.blade,
        hit = { region = regions.wide, collision = parts.slashes_navis, hit_mod = 3 },
        effect = WIDE_SLASH,
        sound = parts.swing,
        step = true,
    },
}

return { ["stepswrd-protoman"] = chip }
