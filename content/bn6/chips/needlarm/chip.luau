--!strict
-- NeedlArm, one of the four arm chips no folder holds: the navi's charged
-- shot becomes the arm's weapon, with a glow at its arm
-- (lib/navi-boost/controller).

local navi_boost = require("../../lib/navi-boost/controller")

-- The arm's charged shot: weapon routine 0x24 (compat's
-- megaman/sticky-charge-24), a number until the weapons are definitions.
local ROUTINE = 0x24
-- The glow's palette.
local PALETTE = 1

-- (The record's extra flags 0x10 and 0x20 have no battle reader, and no
-- name. It has no flags at all, not even `dimming`, though its use is a
-- dimming: it can't cut in on the other side's dimming, and the Beast rush
-- chains it.)
return define.chip {
    id = "needlarm",
    name = "NeedlArm",
    description = "\n\n",
    codes = { "A", "B", "C", "*" },
    element = "null", family = "null", class = "standard",
    rarity = 4, mb = 30, damage = 0, hit_param = 30, lockout = 20,
    library = { number = 0, index = 62, sort = 0 },
    slot_in_limit = 1,
    dimming = navi_boost.arm(ROUTINE, PALETTE),
}
