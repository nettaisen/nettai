--!strict
-- DethPhnx's dimming controller (EXE5's effect object #0x15, 0x080E1E9C): Phoenix's (../phoenix/controllers) with
-- DeathPhoenix (./phoenix).

local controllers = require("../phoenix/controllers")
local phoenix = require("./phoenix")

return controllers.make { id = "dethphnx/controller", name = "DethPhnx", spawn = phoenix.spawn }
