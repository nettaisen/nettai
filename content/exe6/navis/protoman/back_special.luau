--!strict
-- ProtoMan's B+Back specials: `protoman/back-special` (the original's
-- weapon routine 0x34, `sub_80120A6`) raises a Reflector's guard
-- (chips/rflectr/guard, the original's action 0x2B as its subtype 3) that
-- only guards, in the NaviCust Reflect's look; `protoman/back-special-2`
-- (0x30, `sub_8012018`) raises the NaviCust Reflect's own guard
-- (navis/megaman's `megaman/reflect/action`: a hit it guards is fired back
-- as a buster shot), with the damage his stats give. Either holds 20
-- ticks, and his chips wait 40 ticks after it.

local guard = require("../../chips/rflectr/guard")
local navicust = require("../megaman/weapons/shield")
local reflector_shield = require("../../chips/rflectr/shield")

local back_special = {}

local LOCKOUT = 0x28
local TICKS = 0x14

-- The guard that only guards (the original's params word 0x114 with
-- subtype 3).
local GUARD = guard.action {
    id = "protoman/back-special/action",
    ticks = TICKS,
    look = reflector_shield.looks.navicust_reflect,
    heedless = true,
}

local function raise(navi: Object, damage: number)
    navi.attack_element = 0
    navi.charged = 0
    navi.extra = 0
    navi.hit_param = 0
    navi.attack_damage = damage
    navi.attack_lockout = LOCKOUT
end

-- `sub_80120A6`.
local function setup(navi: Object): Action
    raise(navi, 0)
    return GUARD
end

-- `sub_8012018`: the shot's damage is the navi's (NaviStats+0x48).
local function setup_reflect(navi: Object): Action
    raise(navi, battle.navi(navi.alliance).back_special_damage)
    return navicust.reflect_action
end

back_special.weapon = new.weapon {
    id = "protoman/back-special",
    -- The charge table's row 0x34, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 70, 70, 70, 70, 70, 90, 80, 70 },
    setup = setup,
}

back_special.weapon_2 = new.weapon {
    id = "protoman/back-special-2",
    -- The charge table's row 0x30, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 0, 0, 0, 0, 0, 0, 0, 0 },
    setup = setup_reflect,
}

return back_special
