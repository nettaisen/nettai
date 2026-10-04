--!strict
-- CircGun's dimming controller (the original's effect object #0x5E,
-- `sub_80E7568`): the shared controller (@exelib/circgun/controller), BN6's
-- kind with BN6's gun (./gun).

local common = require("@exelib/circgun/controller")
local gun = require("./gun")

local bn6 = common.make("bn6:circgun/controller", gun.kind)

return { hook = bn6.hook, variant = common.variant, make = common.make }
