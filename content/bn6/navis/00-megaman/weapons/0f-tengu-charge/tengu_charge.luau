--!strict
-- TenguCross's charged shot: weapon routine 0xF (`sub_8011C88`). It blows
-- WindRack's gust (action 0x3F, a standard chip's action) as a Wind attack
-- of 40 damage and 20 more per point of buster Attack (up to 5). It leaves
-- the parameters, the charge and the lockout as they were.

local weapon = require("../../../../lib/weapon")

local tengu_charge = {}

local ACTION = 0x3F

-- The attack's element (Wind, secondary element 0x20), variant and hit
-- parameter.
local WIND, VARIANT, HIT_PARAM = 0x20, 1, 0x94

-- `sub_8011C88`.
function tengu_charge.setup(navi: Object): number
    navi.attack_damage = weapon.charge_damage(navi, 0x28, 0x14)
    navi.extra = 0
    navi.hit_param = HIT_PARAM
    navi.variant = VARIANT
    navi.attack_element = WIND
    return ACTION
end

return tengu_charge
