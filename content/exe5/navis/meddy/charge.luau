--!strict
-- Meddy's charged attack (`meddy/charge`): the original's weapon routine 0x3B (0x0800FAA2) sets the damage her level
-- gives (the damage rows' row 10, lib/navi_level) with no bonus, her own counter byte, no element and a plain
-- capsule's parameters (variant 0: no status; its column; not running while dimmed), and starts her own action 7,
-- her throw (./throw).

local capsule = require("../../chips/meddy/capsule")
local navi_level = require("../../lib/navi_level")
local throw = require("./throw")

local charge = {}

local HIT_PARAM = 0x94
-- The damage rows' row 10, by her level.
local DAMAGE: { number } = { 80, 80, 80, 80, 80, 80, 100 }

charge.action = throw.action({ variant = capsule.plain, line = capsule.column, while_dimmed = false }, "meddy/charge/action")

-- 0x0800FAA2
local function setup(navi: Object): Action
    navi.attack_damage = navi_level.damage(navi, DAMAGE)
    navi.extra = 0
    navi.hit_param = HIT_PARAM
    navi.attack_element = 0
    return charge.action
end

charge.weapon = new.weapon {
    id = "meddy/charge",
    -- The charge table's row 0x3B (0x0801CCBA), by Charge 0 to 5: past 4 it reads on into row 0x3C's first entry.
    charge_ticks = { 60, 55, 50, 45, 40, 80 },
    setup = setup,
}

return charge
