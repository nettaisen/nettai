--!strict
-- The instruments' dimming controller (the original's effect object
-- #0x4A): the shared controller (@exelib/instruments/controller), EXE6's
-- kind with EXE6's instrument.

local common = require("@exelib/instruments/controller")
local instrument = require("./instrument")

local instruments = {}

instruments.make = common.make

instruments.hook = common.make("instruments/controller", instrument.kind)

return instruments
