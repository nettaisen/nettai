--!strict
-- SearchMan's charged shot (`searchman/charge`): the original's weapon routine 0x3F (0x0800FB1A) sets the damage his
-- level gives (the damage rows' row 4, lib/navi_level), his own counter byte and no element, and starts action 0x3B,
-- the one SearchSoul's charged shot starts (navis/megaman/forms/searchsoul/charge: the scope marks the enemy navi
-- nearest ahead and five shots follow on its panel, the last one flinching).

local navi_level = require("../../lib/navi_level")
local shots = require("../megaman/forms/searchsoul/charge")

local charge = {}

local HIT_PARAM = 0x8A
-- The damage rows' row 4, by his level: a shot's damage.
local DAMAGE: { number } = { 10, 10, 10, 10, 10, 10, 15 }

-- 0x0800FB1A
local function setup(navi: Object): Action
    navi.attack_damage = navi_level.damage(navi, DAMAGE)
    navi.hit_param = HIT_PARAM
    navi.attack_element = 0
    return shots.action
end

charge.weapon = new.weapon {
    id = "searchman/charge",
    -- The charge table's row 0x3F (0x0801CCE2), by Charge 0 to 5: past 4 it reads on into row 0x40's first entry.
    charge_ticks = { 110, 105, 100, 95, 90, 110 },
    setup = setup,
}

return charge
