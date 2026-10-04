--!strict
-- The instruments' dimming controller (the original's effect object
-- #0x4A): the shared controller (@exelib/instruments/controller), BN6's
-- kind with BN6's instrument.

local common = require("@exelib/instruments/controller")
local instrument = require("./instrument")

local instruments = {}

instruments.make = common.make

instruments.hook = common.make("bn6:instruments/controller", instrument.kind)

return instruments
