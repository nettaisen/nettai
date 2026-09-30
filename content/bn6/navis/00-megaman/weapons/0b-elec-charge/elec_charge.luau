--!strict
-- ElecCross's charged shot: weapon routine 0xB (`sub_8011C38`). It strikes
-- with DolThdr's lightning (action 0x3E, a standard chip's action) as an
-- Elec attack of 40 damage and 20 more per point of buster Attack (up to
-- 5). It leaves the variant, the charge and the lockout as they were.

local weapon = require("../../../../lib/weapon")

local elec_charge = {}

local ACTION = 0x3E

-- The attack's element (Elec) and hit parameter.
local ELEC, HIT_PARAM = 3, 0x94

-- Its parameters (the word 0x10F00): DolThdr's own.
local PARAMS = { 0x00, 0x0F, 0x01, 0x00 }

-- `sub_8011C38`.
function elec_charge.setup(navi: Object): number
    navi.attack_damage = weapon.charge_damage(navi, 0x28, 0x14)
    navi.extra = 0
    navi.hit_param = HIT_PARAM
    navi.attack_element = ELEC
    for i = 1, 4 do
        navi:set_attack_param(i, PARAMS[i])
    end
    return ACTION
end

return elec_charge
