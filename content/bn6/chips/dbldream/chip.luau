--!strict
-- DblDream: two big slashes on the two columns ahead.
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
        id = "dbldream/action",
        blade = parts.blades.sword,
        hit = { region = regions.wide_long, collision = parts.slashes_navis, hit_mod = 1 },
        effect = parts.effects.big,
        sound = parts.sounds.big_swing,
        swings = 2,
    },
}
