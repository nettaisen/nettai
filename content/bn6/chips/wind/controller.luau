--!strict
-- Wind's and Fan's dimming controller (the original's effect object #0x25, `sub_80E309C`;
-- docs/engine/dimming-chip-effects.md §1): the shared one (@exelib/wind/controller) placing BN6's fan (./fan).

local common = require("@exelib/wind/controller")
local fan = require("./fan")

local made = common.make { id = "bn6:wind/controller", spawn_fan = fan.spawn }

-- A chip's `dimming` hook (`sub_80E3128`): the controller placing a fan of `variant`.
return { hook = made.hook }
