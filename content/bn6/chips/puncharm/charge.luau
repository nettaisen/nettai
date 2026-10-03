--!strict
-- PunchArm's charged shot (`puncharm/charge`): the weapon the chip makes
-- the navi's charged shot (lib/navi-boost/controller; the original's weapon
-- routine 0x23, `sub_8011EAC`). It is FireHit's effect (instant chip
-- effect 8, chips/firehit/fist) with fists that stagger (the original's
-- second parameter byte, 3), for 100 Fire. Sticky: a form's own charged
-- shot doesn't replace it, and while the navi has it a Beast buster gives
-- way to the plain one.

local charge = require("../../lib/navi-boost/charge")
local fist = require("../firehit/fist")

-- The attack's element byte: Fire.
local ELEMENT = 1
local DAMAGE = 0x64
-- The fists' hit modifier.
local HIT_MOD = 3

-- `sub_8011EAC`: its effect is the weapon's `instant`, which the instant
-- chips' action runs.
local function setup(navi: Object)
    charge.prepare(navi, ELEMENT, DAMAGE)
    return nil
end

return define.weapon {
    id = "bn6:puncharm/charge",
    charge_ticks = { 80, 80, 80, 80, 80, 100, 100, 100 }, -- by Charge 0..7; past 4, the next routine's row
    setup = setup,
    instant = fist.instant(HIT_MOD),
    sticky = true,
}
