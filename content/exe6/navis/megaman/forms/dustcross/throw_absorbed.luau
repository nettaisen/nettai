--!strict
-- DustCross's buster (`dustcross/throw-absorbed`): the original's weapon
-- routine 0x2B (`sub_8011F8C`), the buster of a navi that absorbs
-- obstacles. It throws the last obstacle absorbed (the buster's shot as a
-- throw, 200 damage), or fires the plain buster when there is none.

local buster = require("../../weapons/buster")

local throw_absorbed = {}

local DAMAGE = 200

-- `sub_8011F8C`'s throw (`sub_8011FCE`'s too): the newest obstacle
-- absorbed goes into the attack; false when the navi has none.
function throw_absorbed.take(navi: Object): boolean
    local look, anim = navi:pop_absorbed()
    if look == nil then
        return false
    end
    -- What it throws, which the game keeps in the action's marker word (the
    -- absorbed list's byte, kind | animation << 4, and the kind's sprite in
    -- the high half): the obstacle's look and animation.
    navi.thrown_look, navi.thrown_anim = look, anim :: number
    navi.attack_damage = DAMAGE
    -- (The game sets the hit parameter to 0x94 and then clears it.)
    navi.hit_param = 0
    navi.attack_element = 0
    navi.charged = 0
    navi.extra = 0
    navi.attack_lockout = 0
    return true
end

-- `sub_8011F8C`.
local function setup(navi: Object): Action
    if throw_absorbed.take(navi) then
        return buster.throw(navi)
    end
    return buster.setup(navi)
end

throw_absorbed.weapon = new.weapon {
    id = "dustcross/throw-absorbed",
    -- The charge table's row 0x2B, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 0, 0, 0, 0, 0, 0, 0, 0 },
    setup = setup,
}

return throw_absorbed
