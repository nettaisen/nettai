--!strict
-- Magnum's dimming controller (EXE5's effect object #0x30, 0x080E3D84; the dimming handler's subtype 15, 0x080E3E04):
-- @exelib/magnum/controller with EXE5's gunner (./gunner).

local common = require("@exelib/magnum/controller")
local gunner = require("./gunner")

return common.make { id = "magnum/controller", gunner = gunner.spawn }
