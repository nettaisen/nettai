--!strict
-- Phoenix's dimming controller (BN5's effect object #0x0D, 0x080E1590): ./controllers' with the phoenix
-- (./phoenix).

local controllers = require("./controllers")
local phoenix = require("./phoenix")

return controllers.make { id = "phoenix/controller", name = "Phoenix", spawn = phoenix.spawn }
