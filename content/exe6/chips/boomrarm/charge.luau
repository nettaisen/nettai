--!strict
-- BoomrArm's charged shot (`boomrarm/charge`): the weapon the chip makes
-- the navi's charged shot (lib/navi_boost/controller; the original's weapon
-- routine 0x26, `sub_8011F10`). It is Boomer's effect (instant chip effect
-- 1, chips/boomer/boomerang: the original's parameter bytes are 0, Boomer's
-- boomerang) for 100 Wood. Sticky: a form's own charged shot doesn't
-- replace it, and while the navi has it a Beast buster gives way to the
-- plain one.

local boomer_chips = require("../boomer")
local charge = require("../../lib/navi_boost/charge")

-- The attack's element byte: Wood.
local ELEMENT = 4
local DAMAGE = 0x64

-- `sub_8011F10`: its effect is the weapon's `instant`, which the instant
-- chips' action runs.
local function setup(navi: Object)
    charge.prepare(navi, ELEMENT, DAMAGE)
    return nil
end

return new.weapon {
    id = "boomrarm/charge",
    charge_ticks = { 100, 100, 100, 100, 100, 90, 80, 70 }, -- by Charge 0..7; past 4, the next routine's row
    setup = setup,
    instant = boomer_chips.boomer.instant,
    sticky = true,
}
