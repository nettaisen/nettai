--!strict
-- Anubis: a statue in front of the user that drains every enemy navi 1 HP
-- each 6 ticks for 3000 ticks, with bubbles over the enemy's area
-- (docs/engine/dimming-chip-effects.md §7).
--
-- Its record stays the pack's for now (PoisPhar's recipe names it): this is
-- its `dimming` hook, which registration by number runs (chips/098-anubis).

local controller = require("./controller")
local statue = require("./statue")

return { dimming = controller.hook(statue.anubis) }
