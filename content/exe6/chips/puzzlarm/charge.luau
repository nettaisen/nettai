--!strict
-- PuzzlArm's charged shot (`puzzlarm/charge`): the weapon the chip makes
-- the navi's charged shot (lib/navi_boost/controller; the original's weapon
-- routine 0x25, `sub_8011EF0`). It is ElcPuls's pulse (chips/elcpuls) with
-- parameters no chip has: no hit modifier, paralysis, 60 ticks, ElcPuls1's
-- look, for 100 Elec. Sticky: a form's own charged shot doesn't replace it,
-- and while the navi has it a Beast buster gives way to the plain one.

local action = require("../elcpuls/action")
local charge = require("../../lib/navi_boost/charge")
local pulse = require("../elcpuls/pulse")
local statuses = require("../../rules/status")

local puzzlarm = {}

-- The attack's element byte: Elec.
local ELEMENT = 3
local DAMAGE = 0x64
-- The pulse's ticks; the navi holds it out 5 less.
local TICKS = 0x3C

-- The pulse (the original's parameter bytes 0, 0x10, 0x3C, 0).
puzzlarm.action = action.action {
    id = "puzzlarm/charge/action",
    pulse = pulse.variant { hit_mod = 0, status = statuses.paralyze_90, ticks = TICKS, palette = 1 },
    hold = TICKS - 5,
}

-- `sub_8011EF0`.
local function setup(navi: Object): Action
    charge.prepare(navi, ELEMENT, DAMAGE)
    return puzzlarm.action
end

puzzlarm.weapon = new.weapon {
    id = "puzzlarm/charge",
    charge_ticks = { 120, 120, 120, 120, 120, 100, 100, 100 }, -- by Charge 0..7; past 4, the next routine's row
    setup = setup,
    sticky = true,
}

return puzzlarm
