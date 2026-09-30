--!strict
-- CrosSwrd: an X-shaped slash ahead, and a second hit on the panel ahead.
--
-- Its record stays the pack's for now (SlashCross charges it, and its charged slash reads the
-- chip's subtype and first parameter from the record, until SlashCross's weapons convert): this is
-- its action, which registration by number runs (docs/design/content-model-v2.md §5.2, "As
-- built").

local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local slash = require("../../lib/swords/slash")

return {
    action = slash.action {
        id = "crosswrd/action",
        blade = parts.blades.sword,
        hit = { region = regions.x, collision = parts.slashes_navis, hit_mod = 3 },
        effect = parts.effects.cross,
        sound = parts.sounds.swing,
        second_hit = slash.CROSS_HIT,
    },
}
