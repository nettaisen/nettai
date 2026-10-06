--!strict
-- NeedlArm's charged shot (`needlarm/charge`): the weapon the chip makes
-- the navi's charged shot (lib/navi_boost/controller; the original's weapon
-- routine 0x24, `sub_8011ED0`). It is AquaNdl1's volley (chips/aquandl:
-- the original's parameter bytes are 0, the first palette) for 40 Aqua.
-- Sticky: a form's own charged shot doesn't replace it, and while the navi
-- has it a Beast buster gives way to the plain one.

local aquandl_chips = require("../aquandl")
local charge = require("../../lib/navi_boost/charge")

-- The attack's element byte: Aqua.
local ELEMENT = 2
local DAMAGE = 0x28

-- `sub_8011ED0`.
local function setup(navi: Object): Action
    charge.prepare(navi, ELEMENT, DAMAGE)
    return assert(aquandl_chips.aquandl1.action)
end

return new.weapon {
    id = "needlarm/charge",
    charge_ticks = { 100, 100, 100, 100, 100, 120, 120, 120 }, -- by Charge 0..7; past 4, the next routine's row
    setup = setup,
    sticky = true,
}
