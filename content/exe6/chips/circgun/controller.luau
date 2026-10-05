--!strict
-- CircGun's dimming controller (the original's effect object #0x5E,
-- `sub_80E7568`): the shared controller (@exelib/circgun/controller), EXE6's
-- kind with EXE6's gun (./gun).

local common = require("@exelib/circgun/controller")
local gun = require("./gun")

local exe6 = common.make("circgun/controller", gun.kind)

return { hook = exe6.hook, variant = common.variant, make = common.make }
