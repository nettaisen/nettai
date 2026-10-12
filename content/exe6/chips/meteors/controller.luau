--!strict
-- Meteors' dimming controller (the original's effect object #0x34): the
-- shared controller (@exelib/meteors/controller), EXE6's kind with EXE6's
-- meteor (./falling_meteor). The chip's `dimming` hook is
-- `controller.hook`.

local common = require("@exelib/meteors/controller")
local falling_meteor = require("./falling_meteor")

local controller = {}

controller.make = common.make

controller.hook = common.make {
    id = "meteors/controller",
    meteor = falling_meteor.kind,
    row = falling_meteor.row,
    targets = common.listed,
}

return controller
