--!strict
-- BugCharg's dimming controller (EXE5's effect object #0x7C, 0x080E98C8; its hook 0x080E9946):
-- @exelib/bugcharg/controller with EXE5's stand-in (./actor).

local actor = require("./actor")
local common = require("@exelib/bugcharg/controller")

return common.make { id = "bugcharg/controller", actor = actor.spawn }
