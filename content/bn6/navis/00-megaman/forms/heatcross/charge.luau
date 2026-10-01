--!strict
-- HeatCross's charged shot (`heatcross/charge`): the original's weapon
-- routine 6 (`sub_8011BA2`). It fires FireBrn's flame (action 0x27, a
-- standard chip's action) as a Fire attack of 30 damage and 20 more per
-- point of buster Attack (up to 5).
--
-- Transitional (docs/design/content-model-v2.md §5.7, "As built"): the form
-- is still the pack's record, which names this weapon by its routine
-- number (the `legacy` marker), and FireBrn's action is still v1, which it
-- names by number until FireBrn converts.

local weapon = require("../../../../lib/weapon")

local heat_charge = {}

-- FireBrn's action (by number, until FireBrn converts).
local FIRE_BURNER_ACTION = 0x27

-- The attack's element (Fire), variant and hit parameter.
local FIRE, VARIANT, HIT_PARAM = 1, 1, 0x94

-- Its parameters (the word 0x23C): FireBrn's own.
local PARAMS = { 0x3C, 0x02, 0x00, 0x00 }

-- `sub_8011BA2`.
local function setup(navi: Object): number
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
    return FIRE_BURNER_ACTION
end

heat_charge.weapon = define.weapon {
    id = "heatcross/charge",
    name = "Heat charge",
    -- The charge table's row 6; Charge 5 reads on into row 7's first
    -- entry, as the original's table does.
    charge_ticks = { 70, 60, 50, 45, 40, 90 },
    setup = setup,
    -- (Transitional: the routine number HeatCross's form names it by.)
    legacy = { routines = { 0x06 } },
}

return heat_charge
