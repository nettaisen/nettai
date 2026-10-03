--!strict
-- BN5's Geddon controller (effect object #0x1D, 0x080E2718): BN6's (@common/geddon/controller) without holding the
-- user's panel (0x080E275C), setting off BN6's quake (@bn6/chips/geddon/quake, BN5's effect #0x1E, the same code:
-- 0x080E27F0; its rows, by the chip's first parameter, crack, break or poison every solid panel).

local common = require("@common/geddon/controller")
local quake = require("@bn6/chips/geddon/quake")

return common.make { id = "bn5:geddon/controller", name = "Geddon", quake = quake.spawn, hold_panel = false }
