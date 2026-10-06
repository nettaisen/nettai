--!strict
-- HubBatc: the navi gets the Hub batch's programs for the rest of the
-- battle: a stronger buster, a bigger custom screen, a B+Back shield, the
-- shoes and the undershirt (lib/navi_boost/controller).

local navi_boost = require("../lib/navi_boost/controller")

-- (The record's extra flags 0x08 and 0x10 have no battle reader, and no
-- name.)
return define.chip {
    id = "hubbatc",
    codes = { "J" },
    element = "null", family = "null", class = "giga",
    rarity = 4, mb = 99, damage = 0, hit_param = 148, lockout = 0,
    flags = { "dimming", "library" },
    library = { number = 309, index = 4, sort = 242 },
    slot_in_limit = 1,
    dimming = navi_boost.hub,
}
