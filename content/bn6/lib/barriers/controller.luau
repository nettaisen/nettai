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
local dimming_chips = require("../dimming")

local barrier_chips = {}

export type State = { barrier: Barrier }

local WAIT = 0x3C

-- `sub_80E3AFC`.
local function effect(me: Object)
    if me.phase_init == 0 then
        local user = me.related1
        if not user then
            error("the barrier chips' controller has no user (sub_801A7CC reads through a null pointer)")
        end
        barriers.raise(user, (me.state :: State).barrier)
        me.timer = WAIT
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        dimming_chips.done(me)
    end
end

barrier_chips.kind = define.kind {
    id = "bn6:barriers/controller",
    pool = "effect",
    state = { barrier = "record:barrier" },
    update = dimming_chips.phases { name = "the barrier chips", effect = effect },
}

-- `sub_80E3B50`: a chip's `dimming` hook, for its barrier. Unlike the
-- other subtypes' spawners, it takes the user's side but not its panel or
-- facing (so the controller stays at panel (0, 0), facing its side's way).
-- (Its position is the spawner's registers, which nothing reads.)
function barrier_chips.hook(barrier: Barrier): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = battle.spawn(barrier_chips.kind, Vec3.zero)
        if not o then
            return nil
        end
        o.element = spec.element
        o.related1 = user
        o.alliance = user.alliance
        o:set_damage_word(spec.damage)
        local s = o.state :: State
        s.barrier = barrier
        return o
    end
end

return barrier_chips
