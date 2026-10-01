--!strict
-- BgDthThd (Falzar's Giga chip): the navi's charged shot becomes the bug
-- death thunder's weapon (lib/navi-boost/controller).

local navi_boost = require("../../lib/navi-boost/controller")

-- The charged shot it gives: weapon routine 0x22 (compat's
-- megaman/sticky-charge-22), a number until the weapons are definitions.
local ROUTINE = 0x22

-- (The record's extra flags 0x08 and 0x10 have no battle reader, and no
-- name.)
return define.chip {
    id = "bgdththd",
    name = "BgDthThd",
    codes = { "V" },
    element = "elec", family = "elec", class = "giga",
    rarity = 4, mb = 80, damage = 0, hit_param = 138, lockout = 0,
    flags = { "dimming", "library" },
    library = { number = 310, index = 5, sort = 36 },
    slot_in_limit = 1,
    dimming = navi_boost.bug(ROUTINE),
}
