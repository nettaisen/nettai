--!strict
-- StepSwrd: a step two panels ahead, a slash on the column ahead, and back. (ProtoMan's own copy is
-- protoman.luau.)
--
-- Its record stays the pack's for now (SlashCross charges it, and its charged slash reads the
-- chip's subtype and first parameter from the record, until SlashCross's weapons convert: its
-- dash): this is its action, which registration by number runs (docs/design/content-model-v2.md
-- §5.2, "As built").

local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local slash = require("../../lib/swords/slash")

return {
    action = slash.action {
        id = "stepswrd/action",
        blade = parts.blades.sword,
        hit = { region = regions.wide, collision = parts.slashes_navis, hit_mod = 3 },
        effect = parts.effects.wide,
        sound = parts.sounds.swing,
        step = true,
    },
}
