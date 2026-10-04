--!strict
-- Geddon's and the capsules' dimming controller (the original's effect
-- object #0x1D, `sub_80E24E4`; docs/engine/dimming-chip-effects.md §12):
-- the usual dimming phases (lib/dimming), with an effect that holds the
-- user's panel and sets off a quake (./quake) over the field, and waits
-- until it is done. The behavior is BN5's too (@exelib/geddon/controller,
-- which BN5's Geddons make without holding the panel).
--
-- A chip's `dimming` hook is `controller.hook(change)`: Geddon's breaks
-- the panels, PrpCapsl's, HealBall's, MagPanl's and the BeastOut records'
-- crack them (the chips' first parameter, 1 or 0, picked the quake's row).

local common = require("@exelib/geddon/controller")
local quake = require("./quake")

return common.make { id = "bn6:geddon/controller", name = "Geddon", quake = quake.spawn, hold_panel = true }
