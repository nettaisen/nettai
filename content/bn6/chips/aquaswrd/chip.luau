--!strict
-- AquaSwrd: an aqua slash on the column ahead.
--
-- Its record stays the pack's for now (SlashCross charges chips 0x4C to 0x4F by number (phase C
-- gives them a trait)): this is its action, which registration by number runs
-- (docs/design/content-model-v2.md §5.1, "As built").

local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local slash = require("../../lib/swords/slash")
local slashes = require("../../navis/00-megaman/forms/slashcross/slashes")

return {
    action = slash.action {
        id = "aquaswrd/action",
        blade = parts.blades.aqua,
        hit = { region = regions.wide, collision = parts.slashes_navis, hit_mod = 3 },
        effect = parts.effects.wide,
        effect_palette = 2,
        sound = parts.sounds.swing,
        charged = slashes.aqua,
    },
}
