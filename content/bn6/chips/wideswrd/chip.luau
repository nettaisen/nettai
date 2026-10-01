--!strict
-- WideSwrd: a slash on the column ahead.
--
-- Its record stays the pack's for now (LifeSrd's Program Advance recipes name it by number): this
-- is its action, which registration by number runs (docs/design/content-model-v2.md §5.1, "As
-- built").

local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local slash = require("../../lib/swords/slash")
local slashes = require("../../navis/megaman/forms/slashcross/slashes")

return {
    action = slash.action {
        id = "wideswrd/action",
        blade = parts.blades.sword,
        hit = { region = regions.wide, collision = parts.slashes_navis, hit_mod = 3 },
        effect = parts.effects.wide,
        sound = parts.sounds.swing,
        charged = slashes.wide,
    },
}
