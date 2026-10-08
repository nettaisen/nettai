--!strict
-- FireSoul's charged shot (`firesoul/charged-shot`): weapon routine 0x0C (0x0800CCAE: 50 damage, fire, counter byte
-- 0x9E, every other attack variable 0, the parameters 0x1E03) and the flame it starts (action 0x19, ./flame), its
-- flames three panels.

local flame = require("./flame")

local charge = {}

charge.action = flame.action { id = "firesoul/charged-shot/action", count = 3 }

-- 0x0800CCAE.
local function setup(navi: Object): Action
    navi.attack_element = 1
    navi.attack_variant = 0
    navi.charged = 0
    navi.attack_lockout = 0
    navi.extra = 0
    navi.hit_param = 0x9E
    navi.attack_damage = 50
    return charge.action
end

charge.weapon = new.weapon {
    id = "firesoul/charged-shot",
    -- The charge table's row 0x0C (0x080180B8 by AIData +0x0F), by Charge 0 to 4; Charge 5 reads on into row 0x0D's
    -- first entry.
    charge_ticks = { 40, 40, 40, 40, 40, 70 },
    setup = setup,
}

return charge
