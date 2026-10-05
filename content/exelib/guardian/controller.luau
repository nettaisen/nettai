--!strict
-- The Guardian chip's dimming controller, as EXE5 and EXE6 share it (EXE6's effect object #0x52, `sub_80E6758`; EXE5's
-- #0x52, 0x080E67DC, the same code; EXE6's docs/engine/dimming-chip-effects.md §5): the usual dimming phases (EXE6's
-- lib/dimming), with an effect that places a Guardian statue on the panel in front of the user if it is free solid
-- ground, striking with the damage plus the bonus, and waits 30 ticks. Each game makes its own
-- (`controller.make`) with its statue's spawn; the chip's `dimming` hook is the made controller's `hook`.

local dimming_chips = require("@exelib/dimming")

local controller = {}

export type State = {
    -- The Atk+ / cross bonus the telop shows (+0x32), which the statue's damage adds.
    bonus: number,
}

export type Spec = {
    -- The kind's id.
    id: string,
    -- The game's statue's spawn (@exelib/guardian/statue's).
    statue: (owner: Object?, x: number, y: number, damage: number, pos: Vec3) -> Object?,
}

local WAIT = 0x1E
-- Where the statue goes: a solid panel with nothing on it.
local SOLID, OCCUPIED = 0x10, 0x0F88_0080

function controller.make(spec: Spec): { hook: (user: Object, dspec: DimmingChipSpec) -> Object? }
    -- `sub_80E679C`: the statue, then 30 ticks.
    local function effect(me: Object)
        if me.phase_init == 0 then
            local x, y = me.panel_x + me.facing, me.panel_y
            if field.check(x, y, SOLID, OCCUPIED) then
                local bonus = (me.state :: State).bonus
                spec.statue(me, x, y, dimming_chips.damage_plus(me, bonus), Vec3.new(y, me.element, bonus))
            end
            me.timer = WAIT
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t == 0 then
            dimming_chips.done(me)
        end
    end

    local kind = define.kind {
        id = spec.id,
        pool = "effect",
        state = { bonus = "u16" },
        -- `sub_80E6758`.
        update = dimming_chips.phases { name = "Guardian's controller", effect = effect },
    }

    -- The chip's `dimming` hook (`sub_80E67E6`).
    local function hook(user: Object, dspec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(kind, user, dspec)
        if o then
            (o.state :: State).bonus = dspec.bonus
        end
        return o
    end

    return { hook = hook }
end

return controller
