--!strict
-- The Mine chip's dimming controller (the original's effect object #0x29,
-- `sub_80E342C`): the shared controller (@exelib/mine/controller), EXE6's
-- kind with EXE6's mine (./land_mine). The chip's `dimming` hook is
-- `controller.hook`.

local common = require("@exelib/mine/controller")
local land_mine = require("./land_mine")

return common.make { id = "mine/controller", mine = land_mine.kind }
