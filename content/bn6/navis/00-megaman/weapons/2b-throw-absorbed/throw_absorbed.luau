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
    local kind, anim = navi:pop_absorbed()
    if kind == nil then
        return buster.setup(navi)
    end
    -- What it throws, as the game keeps it in the action's marker word: the
    -- absorbed list's byte (kind | animation << 4). (The game also puts the
    -- obstacle's sprite in the word's high half; the sprite is the kind's,
    -- data.objects.absorbed_sprites.)
    navi.marker = bit32.band(bit32.bor(kind, bit32.lshift(anim :: number, 4)), 0xFF)
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
