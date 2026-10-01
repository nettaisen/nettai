--!strict
-- HeatCross's charged shot (`heatcross/charge`): the original's weapon
-- routine 6 (`sub_8011BA2`). It burns FireBrn's flame (lib/burner, the
-- original's action 0x27) from HeatCross's burner, as a Fire attack of 30
-- damage and 20 more per point of buster Attack (up to 5): it burns 60
-- ticks and spreads two panels forward, leaving the panels as they were.
-- HeatCross's form definition names it as its charged shot.

local burn = require("../../../../lib/burner/burn")
local weapon = require("../../../../lib/weapon")

local heat_charge = {}

-- The flame (the original's parameter word 0x23C) from HeatCross's burner
-- (its attack variant 1).
local FLAME = burn.action {
    id = "heatcross/charge/action",
    burner = burn.burners.heat,
    flame = { ticks = 0x3C, spread = 2 },
}

-- The attack's element (Fire) and hit parameter.
local FIRE, HIT_PARAM = 1, 0x94

-- `sub_8011BA2`.
local function setup(navi: Object): Action
    navi.attack_damage = weapon.charge_damage(navi, 0x1E, 0x14)
    navi.hit_param = HIT_PARAM
    navi.attack_element = FIRE
    navi.charged = 0
    navi.extra = 0
    navi.attack_lockout = 0
    return FLAME
end

heat_charge.weapon = define.weapon {
    id = "heatcross/charge",
    name = "Heat charge",
    -- The charge table's row 6; Charge 5 reads on into row 7's first
    -- entry, as the original's table does.
    charge_ticks = { 70, 60, 50, 45, 40, 90 },
    setup = setup,
}

return heat_charge
