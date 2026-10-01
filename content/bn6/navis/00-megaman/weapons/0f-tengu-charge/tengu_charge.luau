--!strict
-- TenguCross's charged shot: weapon routine 0xF (`sub_8011C88`). It swings
-- WindRack's action (the original's 0x3F, chips/windrack) with TenguCross's
-- fan (the action's variant 1), as a Wind attack of 40 damage and 20 more
-- per point of buster Attack (up to 5). It leaves the parameters, the
-- charge and the lockout as they were.

local weapon = require("../../../../lib/weapon")
local windrack = require("../../../../chips/windrack/action")

local tengu_charge = {}

-- WindRack's action with TenguCross's fan (its compat key maps it to
-- 0x3F).
local GUST = windrack.action { id = "tengucross/charge/gust", rack = windrack.racks.tengu_fan }

-- The attack's element (Wind, secondary element 0x20) and hit parameter.
local WIND, HIT_PARAM = 0x20, 0x94

-- `sub_8011C88`. (It also sets the attack's variant to 1, which picks the
-- fan: the action's argument here.)
function tengu_charge.setup(navi: Object): Action
    navi.attack_damage = weapon.charge_damage(navi, 0x28, 0x14)
    navi.extra = 0
    navi.hit_param = HIT_PARAM
    navi.attack_element = WIND
    return GUST
end

return tengu_charge
