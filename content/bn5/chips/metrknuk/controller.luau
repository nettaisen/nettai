--!strict
-- MetrKnuk's dimming controller (BN5's effect object #0x76, 0x080E9188): the shared controller
-- (@common/metrknuk/controller), BN5's kind: 16 fists (./fist), 8 ticks apart, every one at a random panel around
-- the enemies (0x080E91F8; never the last one twice in a row), none when no enemy stands (0x080E9270); its fists
-- null (0x080E9268).

local common = require("@common/metrknuk/controller")
local fist = require("./fist")

return common.make {
    id = "bn5:metrknuk/controller",
    fist = fist.spawn,
    first_on_body = false,
    area_fallback = false,
    fists_take_element = false,
}
