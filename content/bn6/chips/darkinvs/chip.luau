--!strict
-- DarkInvs, a dark chip: its navi runs the Cross special, fighting on its
-- own with chips by its HP (the navi's cross-special request;
-- lib/navi-boost/controller).

local navi_boost = require("../../lib/navi-boost/controller")
local invisibl = require("../invisibl/chip")

return define.chip {
    id = "darkinvs",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 4,
    mb = 64,
    damage = 0,
    hit_param = 50,
    lockout = 0,
    flags = { "dimming" },
    extra_flags = { 16, 32 },
    library = { number = 0, index = 69, sort = 0 },
    slot_in_limit = 1,
    dark_substitute = invisibl,
    dimming = navi_boost.dark,
}
