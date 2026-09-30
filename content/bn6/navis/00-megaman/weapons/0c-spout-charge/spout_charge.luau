--!strict
-- SpoutCross's charged shot: weapon routine 0xC (`sub_8011C5E`). It fires
-- Spreader's shot (action 0x25, a standard chip's action) as an Aqua
-- attack of 20 damage and 10 more per point of buster Attack (up to 5). It
-- leaves the charge and the lockout as they were.

local weapon = require("../../../../lib/weapon")

local spout_charge = {}

local ACTION = 0x25

-- The attack's element (Aqua), variant and hit parameter.
local AQUA, VARIANT, HIT_PARAM = 2, 2, 0x94

-- Its parameters (the word 0xD).
local PARAMS = { 0x0D, 0x00, 0x00, 0x00 }

-- `sub_8011C5E`.
function spout_charge.setup(navi: Object): number
    navi.attack_damage = weapon.charge_damage(navi, 0x14, 0x0A)
    navi.extra = 0
    navi.hit_param = HIT_PARAM
    navi.variant = VARIANT
    for i = 1, 4 do
        navi:set_attack_param(i, PARAMS[i])
    end
    navi.attack_element = AQUA
    return ACTION
end

return spout_charge
