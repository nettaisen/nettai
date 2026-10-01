--!strict
-- BugRSwrd (Gregar's Giga chip): the navi's charged shot becomes the bug
-- rise sword's weapon (lib/navi-boost/controller).

local navi_boost = require("../../lib/navi-boost/controller")

-- The charged shot it gives: weapon routine 0x21 (compat's
-- megaman/sticky-charge-21), a number until the weapons are definitions.
local ROUTINE = 0x21

-- (The record's extra flags 0x04 and 0x10 have no battle reader, and no
-- name.)
return define.chip {
    id = "bugrswrd",
    name = "BugRSwrd",
    description = "Use Bug\nFrag with\nDrkSword!",
    codes = { "V" },
    element = "null", family = "sword", class = "giga",
    rarity = 4, mb = 80, damage = 0, hit_param = 168, lockout = 0,
    flags = { "dimming" },
    library = { number = 305, index = 5, sort = 47 },
    slot_in_limit = 1,
    dimming = navi_boost.bug(ROUTINE),
}
