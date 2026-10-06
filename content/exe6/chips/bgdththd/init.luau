--!strict
-- BgDthThd (Falzar's Giga chip): the navi's charged shot becomes the bug
-- death thunder's weapon (lib/navi_boost/controller).

local charge = require("@self/charge")
local navi_boost = require("../lib/navi_boost/controller")

-- (The record's extra flags 0x08 and 0x10 have no battle reader, and no
-- name.)
local chip: Chip = {
    codes = { "V" },
    element = "elec", family = "elec", class = "giga",
    rarity = 4, mb = 80, damage = 0, hit_param = 138, lockout = 0,
    flags = { "dimming", "library" },
    slot_in_limit = 1,
    dimming = navi_boost.bug(charge.weapon),
}

return { bgdththd = chip }
