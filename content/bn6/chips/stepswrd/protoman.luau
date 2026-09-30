--!strict
-- ProtoMan's own StepSwrd: StepSwrd's step and slash, with ProtoMan's
-- damage by his level.
--
-- Its record stays the pack's for now (a link navi's chip is one of chips
-- 0x190 to 0x19A to the ruleset, and its damage is a formula, which
-- definitions take in step 10): this is its action, which registration by
-- number runs (docs/design/content-model-v2.md §5.1, "As built").

local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local slash = require("../../lib/swords/slash")

return {
    action = slash.action {
        id = "stepswrd-protoman/action",
        blade = parts.blades.sword,
        hit = { region = regions.wide, collision = parts.slashes_navis, hit_mod = 3 },
        effect = parts.effects.wide,
        sound = parts.sounds.swing,
        step = true,
    },
}
