--!strict
-- ProtoMan's B+Back (`protoman/back`): the original's weapon routine 0x0B (0x0800F6CC). It raises EXE5's guard (action
-- 0x1F, lib/guard) as the NaviCust Reflect program's (subtype 4: a hit it guards from the front is fired back as the
-- charged shot's projectile, 20 pixels up), for 20 ticks in the Reflect program's look (its params word, 0x114: the
-- ticks, then the look's row), with the damage his stats give (NaviStats +0x48: his fresh stats' 50); his chips wait 40
-- ticks after it. ProtoSoul's (navis/megaman/forms/protosoul/back) is the same guard with a damage of its own.

local guard = require("../../lib/guard")

local back = {}

local TICKS = 0x14
local LOCKOUT = 40

local REFLECT = guard.action {
    id = "protoman/back/action",
    ticks = TICKS,
    look = guard.looks.navicust_reflect,
    counter = { kind = "shot" },
}

-- 0x0800F6CC
local function setup(navi: Object): Action
    navi.attack_element = 0
    navi.charged = 0
    navi.extra = 0
    navi.hit_param = 0
    navi.attack_damage = battle.navi(navi.alliance).back_special_damage
    navi.attack_lockout = LOCKOUT
    return REFLECT
end

back.weapon = new.weapon {
    id = "protoman/back",
    -- The charge table's row 0x0B (0x0801CADA): no charge.
    charge_ticks = { 0, 0, 0, 0, 0, 0 },
    setup = setup,
}

return back
