--!strict
-- The NaviCust programs' shields, B+Back specials: `megaman/shield` and
-- `megaman/shield-2` (the original's weapon routines 0x3B, `sub_801216E`,
-- and 0x8C, `sub_80125AC`, the same) raise a Reflector's guard
-- (chips/rflectr/guard.luau, the original's action 0x2B as its subtype 3)
-- that only guards; `megaman/reflect-2` and `megaman/reflect` (0x3C,
-- `sub_8012194`, and 0x8B, `sub_8012586`) raise it as its subtype 4: a hit
-- it guards is fired back as a 50-damage buster shot. The shield holds 20
-- ticks in the Shield's look or the Reflect's, and the navi's chips wait 40
-- ticks after it.

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
    look = reflector_shield.looks.navicust_shield,
    heedless = true,
}
local REFLECT = guard.action {
    id = "megaman/reflect/action",
    ticks = TICKS,
    look = reflector_shield.looks.navicust_reflect,
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
local function setup(navi: Object): Action
    raise(navi, 0)
    return SHIELD
end

-- `sub_8012194`, `sub_8012586`.
local function setup_counter(navi: Object): Action
    raise(navi, 0x32)
    return REFLECT
end

-- (B+Back specials, never charge routines: their rows lie past the charge
-- table's fifty. Transitional: the routine numbers the NaviCust's programs
-- name them by.)
shield.shield = define.weapon { id = "megaman/shield", name = "Shield", setup = setup, legacy = { routines = { 0x3B } } }
shield.shield_2 = define.weapon { id = "megaman/shield-2", name = "Shield", setup = setup, legacy = { routines = { 0x8C } } }
shield.reflect = define.weapon { id = "megaman/reflect", name = "Reflect", setup = setup_counter, legacy = { routines = { 0x8B } } }
shield.reflect_2 = define.weapon { id = "megaman/reflect-2", name = "Reflect", setup = setup_counter, legacy = { routines = { 0x3C } } }

return shield
