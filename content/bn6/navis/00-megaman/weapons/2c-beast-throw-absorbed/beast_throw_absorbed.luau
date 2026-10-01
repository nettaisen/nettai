--!strict
-- Weapon routine 0x2C (`sub_8011FCE`), DustCross Beast's buster: it throws
-- the last obstacle absorbed (the buster's shot, action 0x11, variant 2,
-- 200 damage), as DustCross's does (weapon 0x2B), or fires the Falzar
-- Beast forms' buster (weapon 3) when there is none.

local buster = require("../buster/weapon")
local falzar_beast_buster = require("../03-falzar-beast-buster/falzar_beast_buster")

local beast_throw_absorbed = {}

local THROW = 2
local DAMAGE = 200

-- `sub_8011FCE`.
function beast_throw_absorbed.setup(navi: Object): Action | number
    local kind, anim = navi:pop_absorbed()
    if kind == nil then
        return falzar_beast_buster.setup(navi)
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

return beast_throw_absorbed
