--!strict
-- BugRSwrd's charged shot (`bugrswrd/charge`): the weapon the chip makes
-- the navi's charged shot (lib/navi-boost/controller; the original's weapon
-- routine 0x21, `sub_8011E40`). It is a sword's slash (the swords' action,
-- lib/swords/slash): with a bug frag, which it spends, DrkSword's slash on
-- the two columns ahead for 200; without one, Sword's on the panel ahead
-- for 80. Sticky: a form's own charged shot doesn't replace it, and while
-- the navi has it a Beast buster gives way to the plain one.

local charge = require("../../lib/navi-boost/charge")
local drksword = require("../drksword")
local sword = require("../sword")

-- The attack's element byte: Null, with the Sword bit.
local ELEMENT = 0x80
local BUG_DAMAGE, PLAIN_DAMAGE = 0xC8, 0x50

-- `sub_8011E40`.
local function setup(navi: Object): Action
    if charge.spend_bug_frag(navi) then
        charge.prepare(navi, ELEMENT, BUG_DAMAGE)
        return drksword.action
    end
    charge.prepare(navi, ELEMENT, PLAIN_DAMAGE)
    return sword.action
end

return define.weapon {
    id = "bugrswrd/charge",
    charge_ticks = { 120, 120, 120, 120, 120, 200, 200, 200 }, -- by Charge 0..7; past 4, the next routine's row
    setup = setup,
    sticky = true,
}
