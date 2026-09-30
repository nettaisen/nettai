--!strict
-- PoisSeed: a seed that turns the panels it lands on into poison.
--
-- Its record stays the pack's for now (PoisPhar's Program Advance recipe names it by number): this
-- is its action, which registration by number runs (docs/design/content-model-v2.md §5.1, "As
-- built").

local seed = require("../../lib/bombs/seed")
local throw = require("../../lib/bombs/throw")

return {
    action = throw.action {
        id = "poisseed/action",
        held = seed.held(3),
        held_anim = 8,
        thrower = seed.thrower(seed.variant {
            element = 0,
            palette = 3,
            panel = "poison",
            effect = seed.sprout,
            sound = asset.sound("panel-poison"),
        }),
    },
}
