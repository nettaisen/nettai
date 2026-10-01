--!strict
-- DblHero: the Program Advance of WideBlde, LongBlde and ProtoMn[SP]: a
-- cut-in in which MegaMan and ProtoMan stand in for the user, MegaMan's
-- ten volleys hitting every enemy panel (docs/engine/dimming-chip-effects.md
-- §17).
--
-- Its record stays the pack's for now (its Program Advance recipe comes to
-- definitions with the chips it names): this is its `dimming` hook, which
-- registration by number runs (chips/158-dblhero).

local controller = require("./controller")

return { dimming = controller.hook }
