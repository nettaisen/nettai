--!strict
-- Weapon routine 0x2B (`sub_8011F8C`), the buster of a navi that absorbs
-- obstacles: it throws the last obstacle absorbed (the buster's shot,
-- action 0x11, variant 2, 200 damage), or fires the plain buster when
-- there is none.

local buster = require("../buster/weapon")

local throw_absorbed = {}

local THROW = 2
local DAMAGE = 200

-- `sub_8011F8C`.
function throw_absorbed.setup(navi: Object): Action
    local look, anim = navi:pop_absorbed()
    if look == nil then
        return buster.setup(navi)
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
    navi.variant = THROW
    return buster.shot
end

return throw_absorbed
