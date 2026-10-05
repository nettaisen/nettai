--!strict
-- BugRSwrd (Gregar's Giga chip): the navi's charged shot becomes the bug
-- rise sword's weapon (lib/navi-boost/controller).

local charge = require("@self/charge")
local navi_boost = require("../lib/navi-boost/controller")

-- (The record's extra flags 0x04 and 0x10 have no battle reader, and no
-- name.)
return define.chip {
    id = "bugrswrd",
    codes = { "V" },
    element = "null", family = "sword", class = "giga",
    rarity = 4, mb = 80, damage = 0, hit_param = 168, lockout = 0,
    flags = { "dimming" },
    library = { number = 305, index = 5, sort = 47 },
    slot_in_limit = 1,
    dimming = navi_boost.bug(charge),
}
