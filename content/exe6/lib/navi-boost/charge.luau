--!strict
-- What the chips' charged shots share (the weapons BugRSwrd, BgDthThd and
-- the arm chips make the navi's charged shot: the original's weapon
-- routines 0x21 to 0x26, `sub_8011E40` to `sub_8011F10`). Each setup starts
-- alike: not charged, no Atk+ bonus, a chip lockout of 20 ticks after it,
-- the counter byte 0x14.

local exe6 = require("../../rules/api")

local charge = {}

local LOCKOUT = 0x14
local HIT_PARAM = 0x14

-- The attack's variables every one of them sets, with its `element` (the
-- attack's element byte) and `damage`.
function charge.prepare(navi: Object, element: number, damage: number)
    navi.charged = 0
    navi.extra = 0
    navi.attack_lockout = LOCKOUT
    navi.hit_param = HIT_PARAM
    navi.attack_element = element
    navi.attack_damage = damage
end

-- `sub_800F4A8`, `sub_800F4B2`: a bug frag of the navi's side goes, if it
-- has one. Whether it had.
function charge.spend_bug_frag(navi: Object): boolean
    return exe6.spend_bug_frags(navi.alliance, 1)
end

return charge
