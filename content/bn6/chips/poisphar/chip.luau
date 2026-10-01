--!strict
-- PoisPhar: Anubis's statue (chips/anubis) draining every tick for 1441
-- ticks; the Program Advance of Anubis and two PoisSeeds
-- (docs/engine/dimming-chip-effects.md §7).
--
-- Its record stays the pack's for now (its recipe comes to definitions with
-- the chips it names): this is its `dimming` hook, which registration by
-- number runs (chips/098-anubis).

local controller = require("../anubis/controller")
local statue = require("../anubis/statue")

return { dimming = controller.hook(statue.poisphar) }
