--!strict
-- BugCurse's dimming controller (BN5's effect object #0x56, 0x080E6AC8; BN5's own): the usual dimming phases
-- (@common/dimming), with an effect (0x080E6B0C) that lays a curse (./curse) on the enemy navi's panel, unless the
-- enemy navi is invisible, submerged, flashing or bubbled (or there is none), and waits on it; then the undim.

local curse = require("./curse")
local dimming_chips = require("@common/dimming")

export type State = {
    -- The curse is still at work (+0x0C, the flag it was given).
    waiting: boolean,
}

local controller = {}

-- What keeps the curse off the enemy navi (0x080E6B84: 0x80000206).
local UNTOUCHED: { StatusFlag } = { "invisible", "submerged", "flashing", "bubbled" }

local function untouched(o: Object): boolean
    for _, f in UNTOUCHED do
        if o:status(f) then
            return true
        end
    end
    return false
end

-- 0x080E6B0C: on its first tick the curse (60 ticks on the timer, which nothing reads); then the undim once no curse
-- is at work.
local function effect(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        local enemy = battle.player(bit32.bxor(me.alliance, 1))
        if enemy and not untouched(enemy) then
            if curse.spawn(me, enemy.panel_x, enemy.panel_y, me.element, bit32.bor(me.damage, bit32.lshift(me.stamina, 16))) then
                s.waiting = true
            end
        end
        me.timer = 0x3C
        me.phase_init = 4
    end
    if not s.waiting then
        dimming_chips.done(me)
    end
end

controller.kind = define.kind {
    id = "bn5:bugcurse/controller",
    pool = "effect",
    state = { waiting = "bool" },
    -- 0x080E6AC8.
    update = dimming_chips.phases { name = "BugCurse", effect = effect },
}

-- 0x080E6B62: BugCurse's `dimming` hook (only the alliance byte of its user's).
function controller.hook(user: Object, spec: DimmingChipSpec): Object?
    return dimming_chips.spawn(controller.kind, user, spec, true)
end

return controller
