--!strict
-- The B+Back special of the patch card magntman
-- (`patch-cards/magntman/back-special`): the immobilizer (instant effect 9,
-- lib/instant/immobilize) ahead of the navi, then a 120-tick cooldown before
-- the next B+Back. The original's weapon routine 0x83 (`sub_8012526`).

local immobilize = require("../../lib/instant/immobilize")

-- The cooldown (the attack's lockout byte).
local COOLDOWN = 0x78

-- `sub_8012526`: no damage; its effect is the weapon's `instant`, which the
-- instant chips' action runs.
local function setup(navi: Object)
    navi.attack_lockout = COOLDOWN
    navi.attack_damage = 0
    navi.hit_param = 0
    return nil
end

return new.weapon {
    id = "patch-cards/magntman/back-special",
    -- The charge table's row 0x83, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 0, 0, 0, 0, 0, 180, 170, 160 },
    setup = setup,
    instant = immobilize.instant,
}
