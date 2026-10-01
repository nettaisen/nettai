--!strict
-- The barrier chips' dimming controller (the original's effect object
-- #0x2F, `sub_80E3AB8`; docs/engine/dimming-chips.md §3.1): Barrier,
-- Barr100, Barr200, BblWrap and LifeAur. The usual dimming phases
-- (lib/dimming), with an effect (`sub_80E3AFC`) that raises the chip's
-- barrier on the user with its visual (barriers.luau), then waits 61
-- ticks.
--
-- A chip's `dimming` hook is `barrier_chips.hook(barrier)`.

local barriers = require("./barriers")
local controller = require("../dimming/controller")

local barrier_chips = {}

export type State = { barrier: Barrier }

local WAIT = 0x3C

-- `sub_80E3AFC`.
local function effect(me: Object)
    if me.phase_init == 0 then
        local user = controller.user(me, "sub_801A7CC")
        barriers.raise(user, (me.state :: State).barrier)
        me.timer = WAIT
        me.phase_init = 4
    end
    if controller.count_to_minus_one(me) then
        controller.undim(me)
    end
end

barrier_chips.kind = define.kind {
    id = "barriers/controller",
    pool = "effect",
    state = { barrier = "record:barrier" },
    update = controller.update("the barrier chips' controller", effect),
}

-- `sub_80E3B50`: a chip's `dimming` hook, for its barrier. (The spawner
-- takes the user's side but not its panel or facing.)
function barrier_chips.hook(barrier: Barrier): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = controller.spawn(barrier_chips.kind, user, spec, { panel = false, flip = false })
        if o then
            (o.state :: State).barrier = barrier
        end
        return o
    end
end

return barrier_chips
