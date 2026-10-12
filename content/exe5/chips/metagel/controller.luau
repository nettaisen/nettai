--!strict
-- MetaGel's dimming controller (EXE5's effect object #0x21, 0x080E2D98; the dimming handler's subtype 6, 0x080E2E9C):
-- @exelib/metagel/controller with EXE5's gel (./gel), each dropped past the user's area (0x0800BA34).

local common = require("@exelib/metagel/controller")
local gel = require("./gel")

return common.make { id = "metagel/controller", gel = gel.spawn, front = common.past_own_area }
