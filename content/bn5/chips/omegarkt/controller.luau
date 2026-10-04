--!strict
-- OmegaRkt's dimming controller (BN5's effect object #0x79, 0x080E95BC; BN5's own, on the dimming chips' skeleton): the
-- usual dimming phases (@exelib/dimming), with an effect (0x080E9600) that launches a rocket (./rocket) from its side's
-- back column, middle row, its damage the chip's and the Atk+ bonus, and waits on it; then the undim.

local dimming_chips = require("@exelib/dimming")
local rocket = require("./rocket")

export type State = {
    -- The Atk+ / cross bonus (object +0x32).
    bonus: number,
    -- The rocket at work (+0x60), which clears it as it goes.
    rocket: Object?,
}

local controller = {}

-- 0x080E9600: on its first tick the rocket (30 ticks on the timer, which nothing reads); then the undim once no
-- rocket is at work.
local function effect(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        local x = bit32.bxor(me.alliance, me.flip) * 5 + 1
        s.rocket = rocket.spawn(me, x, 2, me.element, dimming_chips.damage_plus(me, s.bonus))
        me.timer = 0x1E
        me.phase_init = 4
    end
    if s.rocket == nil then
        dimming_chips.done(me)
    end
end

controller.kind = define.kind {
    id = "omegarkt/controller",
    pool = "effect",
    state = { bonus = "u16", rocket = "object" },
    -- 0x080E95BC.
    update = dimming_chips.phases { name = "OmegaRkt", effect = effect },
}

-- 0x080E9642: OmegaRkt's `dimming` hook.
function controller.hook(user: Object, spec: DimmingChipSpec): Object?
    local o = dimming_chips.spawn(controller.kind, user, spec)
    if o then
        (o.state :: State).bonus = spec.bonus
    end
    return o
end

return controller
