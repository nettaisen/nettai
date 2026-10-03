--!strict
-- BN5's Invisibl's dimming controller (effect object #0x5D, BN6's code: @common/invisibl/controller), BN5's kind;
-- its sound BN6's.

local common = require("@common/invisibl/controller")

return common.make { id = "bn5:invisibl/controller", sound = asset.sound("bn6:invisible") }
