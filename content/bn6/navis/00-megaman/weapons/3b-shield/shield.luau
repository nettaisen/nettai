--!strict
-- The NaviCust programs' shields: weapon routines 0x3B (`sub_801216E`, the
-- Shield program's B+Back special) and 0x8C (`sub_80125AC`, the same),
-- which raise Reflector's shield (action 0x2B) as its subtype 3 (it only
-- guards), and 0x3C (`sub_8012194`, the Reflect program's) and 0x8B
-- (`sub_8012586`) as its subtype 4 (a hit it guards is fired back as a
-- 50-damage buster shot). The shield holds 20 ticks (Param1) in look 2
-- (Param2) or 1, and the navi's chips wait 40 ticks after it.

local shield = {}

local ACTION = 0x2B
local LOCKOUT = 0x28

local function raise(navi: Object, variant: number, damage: number, params: { number }): number
    navi.attack_element = 0
    navi.variant = variant
    navi.charged = 0
    navi.extra = 0
    navi.hit_param = 0
    navi.attack_damage = damage
    for i = 1, 4 do
        navi:set_attack_param(i, params[i])
    end
    navi.attack_lockout = LOCKOUT
    return ACTION
end

-- `sub_801216E`, `sub_80125AC`: the params word 0x214.
function shield.setup(navi: Object): number
    return raise(navi, 3, 0, { 0x14, 0x02, 0x00, 0x00 })
end

-- `sub_8012194`, `sub_8012586`: the params word 0x114.
function shield.setup_counter(navi: Object): number
    return raise(navi, 4, 0x32, { 0x14, 0x01, 0x00, 0x00 })
end

return shield
