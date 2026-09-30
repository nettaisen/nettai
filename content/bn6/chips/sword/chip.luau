--!strict
-- Sword: a slash on the panel ahead.
--
-- Its record stays the pack's for now (LifeSrd's Program Advance recipes name it by number): this
-- is its action, which registration by number runs (docs/design/content-model-v2.md §5.1, "As
-- built").

local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local slash = require("../../lib/swords/slash")

return {
    action = slash.action {
        id = "sword/action",
        blade = parts.blades.sword,
        hit = { region = regions.single, collision = parts.slashes_navis, hit_mod = 3 },
        effect = parts.effects.slash,
        sound = parts.sounds.swing,
    },
}
