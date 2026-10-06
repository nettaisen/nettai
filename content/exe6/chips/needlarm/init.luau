--!strict
-- NeedlArm, one of the four arm chips no folder holds: the navi's charged
-- shot becomes the arm's weapon, with a glow at its arm
-- (lib/navi_boost/controller).

local charge = require("@self/charge")
local navi_boost = require("../lib/navi_boost/controller")

-- The glow's palette.
local PALETTE = 1

-- (The record's extra flags 0x10 and 0x20 have no battle reader, and no
-- name. It has no flags at all, not even `dimming`, though its use is a
-- dimming: it can't cut in on the other side's dimming, and the Beast rush
-- chains it.)
local chip: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null", family = "null", class = "standard",
    rarity = 4, mb = 30, damage = 0, hit_param = 30, lockout = 20,
    slot_in_limit = 1,
    dimming = navi_boost.arm(charge, PALETTE),
}

return { needlarm = chip }
