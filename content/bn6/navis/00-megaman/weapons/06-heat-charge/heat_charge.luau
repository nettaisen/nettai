--!strict
-- HeatCross's charged shot: weapon routine 6 (`sub_8011BA2`). It fires
-- FireBrn's flame (action 0x27, a standard chip's action) as a Fire attack
-- of 30 damage and 20 more per point of buster Attack (up to 5).

local weapon = require("../../../../lib/weapon")

local heat_charge = {}

local ACTION = 0x27

-- The attack's element (Fire), variant and hit parameter.
local FIRE, VARIANT, HIT_PARAM = 1, 1, 0x94

-- Its parameters (the word 0x23C): FireBrn's own.
local PARAMS = { 0x3C, 0x02, 0x00, 0x00 }

-- `sub_8011BA2`.
function heat_charge.setup(navi: Object): number
    navi.attack_damage = weapon.charge_damage(navi, 0x1E, 0x14)
    navi.hit_param = HIT_PARAM
    navi.attack_element = FIRE
    navi.variant = VARIANT
    navi.charged = 0
    navi.extra = 0
    navi.attack_lockout = 0
    for i = 1, 4 do
        navi:set_attack_param(i, PARAMS[i])
    end
    return ACTION
end

return heat_charge
