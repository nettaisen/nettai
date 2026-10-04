--!strict
-- SpoutCross's charged shot (`spoutcross/charge`): the original's weapon
-- routine 0xC (`sub_8011C5E`). It fires the Spreaders' action (the
-- original's 0x25, chips/spreadr) from its water cannon, low, with a
-- splash that hits the panel it finds and the one behind it, as an Aqua
-- attack of 20 damage and 10 more per point of buster Attack (up to 5). It
-- leaves the charge and the lockout as they were.

local bullet = require("../../../../objects/bullet/bullet")
local spreader = require("../../../../chips/spreadr/action")
local weapon = require("../../../../lib/weapon")

local charge = {}

-- The Spreaders' action as the original's variant 2 (the water cannon,
-- fired from the ground: `dword_80ECC54`, `dword_80ECC5C`) with the spout's
-- bullet (the parameter word 0xD). (Its compat key maps it to 0x25.)
local SHOT = spreader.action {
    id = "spoutcross/charge/action",
    gun = spreader.guns.water_cannon,
    z = 0,
    bullet = bullet.variants.spout,
}

-- The attack's element (Aqua) and hit parameter.
local AQUA, HIT_PARAM = 2, 0x94

-- `sub_8011C5E`.
local function setup(navi: Object): Action
    navi.attack_damage = weapon.charge_damage(navi, 0x14, 0x0A)
    navi.extra = 0
    navi.hit_param = HIT_PARAM
    navi.attack_element = AQUA
    return SHOT
end

charge.weapon = define.weapon {
    id = "spoutcross/charge",
    -- The charge table's row 0xC, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 60, 50, 40, 30, 20, 60, 60, 60 },
    setup = setup,
}

return charge
