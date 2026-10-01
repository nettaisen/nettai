--!strict
-- BgDthThd's charged shot (`bgdththd/charge`): the weapon the chip makes
-- the navi's charged shot (lib/navi-boost/controller; the original's weapon
-- routine 0x22, `sub_8011E78`). It is Thunder's shot (chips/thunder/shoot):
-- with a bug frag, which it spends, a fast ball that homes in for 12 panels
-- (DarkThnd's big one) for 200; without one, Thunder's own for 40. Sticky:
-- a form's own charged shot doesn't replace it, and while the navi has it a
-- Beast buster gives way to the plain one.

local charge = require("../../lib/navi-boost/charge")
local shoot = require("../thunder/shoot")
local statuses = require("../../rules/status")
local thunder = require("../thunder/chip")

local bgdththd = {}

-- The attack's element byte: Elec.
local ELEMENT = 3
local BUG_DAMAGE, PLAIN_DAMAGE = 0xC8, 0x28

-- The shot with a bug frag (the original's parameter bytes 2, 12, 0x10): a
-- fast ball of 12 panels that paralyzes.
bgdththd.action = shoot.action {
    id = "bgdththd/charge/action",
    ball = { fast = true, panels = 0x0C, status = statuses.paralyze_90 },
}

-- `sub_8011E78`. (Without a bug frag the parameter bytes are 1, 5, 0x10:
-- Thunder's.)
local function setup(navi: Object): Action
    if charge.spend_bug_frag(navi) then
        charge.prepare(navi, ELEMENT, BUG_DAMAGE)
        return bgdththd.action
    end
    charge.prepare(navi, ELEMENT, PLAIN_DAMAGE)
    return thunder.action
end

bgdththd.weapon = define.weapon {
    id = "bgdththd/charge",
    name = "BgDthThd",
    charge_ticks = { 200, 200, 200, 200, 200, 80, 80, 80 }, -- by Charge 0..7; past 4, the next routine's row
    setup = setup,
    sticky = true,
}

return bgdththd
