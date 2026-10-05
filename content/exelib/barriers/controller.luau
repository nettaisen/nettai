--!strict
-- The barrier chips' dimming controller, as EXE5 and EXE6 share it (EXE6's effect object #0x2F, `sub_80E3AB8`; EXE5's
-- #0x2F, 0x080E3CCC, the same code; EXE6's docs/engine/dimming-chips.md §3.1): Barrier, Barr100, Barr200, BblWrap and
-- LifeAur. The usual dimming phases (EXE6's lib/dimming), with an effect (`sub_80E3AFC`) that raises the chip's
-- barrier on the user with its visual, then waits 61 ticks. Each game makes its own (`controller.make`) with its
-- barriers' raise; a chip's `dimming` hook is the made controller's `hook(barrier)`.

local dimming_chips = require("@exelib/dimming")

local controller = {}

export type State = { barrier: Barrier }

export type Spec = {
    -- The kind's id.
    id: string,
    -- The game's barriers' raise (EXE6's lib/barriers/barriers'): `b` on `navi`, with its visual.
    raise: (navi: Object, b: Barrier) -> (),
}

local WAIT = 0x3C

function controller.make(spec: Spec): { kind: Kind, hook: (barrier: Barrier) -> (user: Object, dspec: DimmingChipSpec) -> Object? }
    -- `sub_80E3AFC`.
    local function effect(me: Object)
        if me.phase_init == 0 then
            local user = me.related1
            if not user then
                error("the barrier chips' controller has no user (sub_801A7CC reads through a null pointer)")
            end
            spec.raise(user, (me.state :: State).barrier)
            me.timer = WAIT
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t < 0 then
            dimming_chips.done(me)
        end
    end

    local kind = define.kind {
        id = spec.id,
        pool = "effect",
        state = { barrier = "record:barrier" },
        update = dimming_chips.phases { name = "the barrier chips", effect = effect },
    }

    -- `sub_80E3B50`: a chip's `dimming` hook, for its barrier. Unlike the other subtypes' spawners, it takes the
    -- user's side but not its panel or facing (so the controller stays at panel (0, 0), facing its side's way). (Its
    -- position is the spawner's registers, which nothing reads.)
    local function hook(barrier: Barrier): (user: Object, dspec: DimmingChipSpec) -> Object?
        return function(user: Object, dspec: DimmingChipSpec): Object?
            local o = battle.spawn(kind, Vec3.zero)
            if not o then
                return nil
            end
            o.element = dspec.element
            o.related1 = user
            o.alliance = user.alliance
            o:set_damage_word(dspec.damage)
            local s = o.state :: State
            s.barrier = barrier
            return o
        end
    end

    return { kind = kind, hook = hook }
end

return controller
