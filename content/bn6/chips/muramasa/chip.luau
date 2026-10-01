--!strict
-- Muramasa: a slash on the two panels ahead, as strong as the HP its user lost.
--
-- Its record stays the pack's for now (its damage is a formula, which definitions take in step
-- 10): this is its action, which registration by number runs (docs/design/content-model-v2.md
-- §5.1, "As built").

local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local slash = require("../../lib/swords/slash")
local slashes = require("../../navis/megaman/forms/slashcross/slashes")

return {
    action = slash.action {
        id = "muramasa/action",
        blade = parts.blades.sword,
        hit = { region = regions.long, collision = parts.slashes_navis, hit_mod = 3 },
        effect = parts.effects.muramasa,
        sound = parts.sounds.swing,
        charged = slashes.muramasa,
    },
}
