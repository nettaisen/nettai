--!strict
-- ProtoSoul's B+Back (`protosoul/back`): the original's weapon routine 4 (0x0800F634). It raises BN5's guard (action
-- 0x1F, lib/guard) as the NaviCust Reflect program's (subtype 4: a hit it guards from the front is fired back as the
-- charged shot's projectile, 20 pixels up, with 50 damage), for 20 ticks (its params word's first byte, 0x114), in
-- the Reflect program's look (the word's second byte: row 1), or row 9's with the Chaos Unison charge armed (0x914);
-- his chips wait 40 ticks after it.

local guard = require("../../../../lib/guard/guard")

local back = {}

local TICKS = 0x14
local DAMAGE = 50
local LOCKOUT = 40

local REFLECT = guard.action {
    id = "protosoul/back/action",
    ticks = TICKS,
    look = guard.looks.navicust_reflect,
    counter = { kind = "shot" },
}
local CHAOS_REFLECT = guard.action {
    id = "protosoul/back/chaos-action",
    ticks = TICKS,
    look = guard.looks.chaos_reflect,
    counter = { kind = "shot" },
}

-- 0x0800F634
local function setup(navi: Object): Action
    navi.attack_element = 0
    navi.charged = 0
    navi.extra = 0
    navi.hit_param = 0
    navi.attack_damage = DAMAGE
    navi.attack_lockout = LOCKOUT
    return if navi.chaos_armed then CHAOS_REFLECT else REFLECT
end

back.weapon = define.weapon {
    id = "protosoul/back",
    -- The charge table's row 4 (0x0801CA94), by Charge 0 to 5: past 4 it reads on into row 5's first entry.
    charge_ticks = { 180, 180, 180, 180, 180, 30 },
    setup = setup,
}

return back
