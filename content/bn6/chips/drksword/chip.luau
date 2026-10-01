--!strict
-- DrkSword: a dark big slash on the two columns ahead (a dark chip: without bug frags, its user gets a Sword).
--
-- Its record stays the pack's for now (the ruleset's dark chips are chips 0x11E to 0x122 (phase C
-- gives them a trait)): this is its action, which registration by number runs
-- (docs/design/content-model-v2.md §5.1, "As built").

local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local slash = require("../../lib/swords/slash")
local slashes = require("../../navis/00-megaman/forms/slashcross/slashes")

return {
    action = slash.action {
        id = "drksword/action",
        blade = parts.blades.sword,
        hit = { region = regions.wide_long, collision = parts.slashes_navis, hit_mod = 3 },
        effect = parts.effects.dark,
        sound = parts.sounds.big_swing,
        charged = slashes.dark,
    },
}
