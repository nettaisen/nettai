--!strict
-- ElecCross's charged shot: weapon routine 0xB (`sub_8011C38`). It strikes
-- with DolThdr's lightning (the original's action 0x3E, chips/dolthdr),
-- the navi holding up the buster arm, as an Elec attack of 40 damage and
-- 20 more per point of buster Attack (up to 5). It leaves the variant, the
-- charge and the lockout as they were.

local dolthdr = require("../../../../chips/dolthdr/action")
local weapon = require("../../../../lib/weapon")

local elec_charge = {}

-- DolThdr's action with its parameters (the word 0x10F00): the thunder's
-- 15 ticks, and the buster arm held up rather than the doll. (Its compat
-- key maps it to 0x3E.)
local THUNDER = dolthdr.action { id = "eleccross/charge/thunder", ticks = 0x0F, holds_arm = true }

-- The attack's element (Elec) and hit parameter.
local ELEC, HIT_PARAM = 3, 0x94

-- `sub_8011C38`.
function elec_charge.setup(navi: Object): Action
    navi.attack_damage = weapon.charge_damage(navi, 0x28, 0x14)
    navi.extra = 0
    navi.hit_param = HIT_PARAM
    navi.attack_element = ELEC
    return THUNDER
end

return elec_charge
