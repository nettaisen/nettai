--!strict
-- The NaviCust programs' shields: weapon routines 0x3B (`sub_801216E`, the
-- Shield program's B+Back special) and 0x8C (`sub_80125AC`, the same),
-- which raise a Reflector's guard (chips/rflectr/guard.luau, the
-- original's action 0x2B as its subtype 3) that only guards, and 0x3C
-- (`sub_8012194`, the Reflect program's) and 0x8B (`sub_8012586`) as its
-- subtype 4: a hit it guards is fired back as a 50-damage buster shot. The
-- shield holds 20 ticks in the Shield's look or the Reflect's, and the
-- navi's chips wait 40 ticks after it.

local guard = require("../../../../chips/rflectr/guard")
local reflector_shield = require("../../../../chips/rflectr/shield")

local shield = {}

local LOCKOUT = 0x28
-- How long the shield holds.
local TICKS = 0x14

-- The guards (the original's action 0x2B with the params words 0x214 and
-- 0x114).
local SHIELD = guard.action {
    id = "megaman/shield/action",
    ticks = TICKS,
    look = reflector_shield.looks[2],
    heedless = true,
}
local REFLECT = guard.action {
    id = "megaman/reflect/action",
    ticks = TICKS,
    look = reflector_shield.looks[1],
    counter = "shot",
}

local function raise(navi: Object, damage: number)
    navi.attack_element = 0
    navi.charged = 0
    navi.extra = 0
    navi.hit_param = 0
    navi.attack_damage = damage
    navi.attack_lockout = LOCKOUT
end

-- `sub_801216E`, `sub_80125AC`.
function shield.setup(navi: Object): Action
    raise(navi, 0)
    return SHIELD
end

-- `sub_8012194`, `sub_8012586`.
function shield.setup_counter(navi: Object): Action
    raise(navi, 0x32)
    return REFLECT
end

return shield
