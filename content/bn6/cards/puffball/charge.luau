--!strict
-- The charged shot of the patch card puffball (`patch-cards/puffball/charge`):
-- a bubble spread. The original's weapon routine 0x5F (`sub_80123AA`) fires
-- the Spreaders' action (0x25, chips/spreadr) as SpoutCross's charged shot
-- does, from the water cannon, low, with the bullet's row 7: a splash over
-- the 3x3 around the panel it finds, 70 Aqua damage.

local bullet = require("../../objects/bullet")
local spreader = require("../../chips/spreadr/action")

-- The Spreaders' action as the original's variant 2 (the water cannon,
-- fired from the ground) with the bubble's bullet (the parameter word 7).
-- (Its compat key maps it to 0x25.)
local SHOT = spreader.action {
    id = "patch-cards/puffball/charge/action",
    gun = spreader.guns.water_cannon,
    z = 0,
    bullet = bullet.variants.bubble,
}

-- The attack: 70 damage, hit parameter 0x0A, Aqua.
local DAMAGE, HIT_PARAM, AQUA = 0x46, 0x0A, 2

-- `sub_80123AA`.
local function setup(navi: Object): Action
    navi.attack_damage = DAMAGE
    navi.hit_param = HIT_PARAM
    navi.attack_element = AQUA
    return SHOT
end

return define.weapon {
    id = "patch-cards/puffball/charge",
    -- The charge table's row 0x5F, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 100, 90, 80, 75, 70, 120, 110, 100 },
    setup = setup,
}
