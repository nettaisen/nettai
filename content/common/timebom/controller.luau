--!strict
-- The TimeBom chips' dimming controller, as BN5 and BN6 share it (BN6's
-- effect object #0x27, `sub_80E31D8`): the usual dimming phases
-- (@common/dimming), with an effect that sets the chip's countdown bomb
-- (./countdown) where the game places it (BN6's: the first free enemy panel
-- in front of the user), and waits 31 ticks.
--
-- A chip's `dimming` hook (`sub_80E3242`, `off_802CCB4[10]`) is
-- `controller.hook(variant)`: the bomb the original's first chip
-- parameter picked (0 TimeBom1-3's, 1 TimeBom+'s) is the hook's argument.
-- Each game's controller places its bomb its own way (`controller.make`).

local dimming_chips = require("../dimming")

export type State = {
    -- The Atk+ / cross bonus the telop shows (object +0x32), which adds to
    -- the bomb's damage.
    bonus: number,
    -- The bomb the chip sets.
    variant: CountdownVariant,
}

local WAIT = 0x1E

-- A game's controller (kind `id`) that sets its bomb with `place`: its
-- kind, and the chips' hook of a bomb variant.
local function make(id: string, place: (me: Object) -> ())
    -- `sub_80E321C`: the bomb, then 31 ticks.
    local function effect(me: Object)
        if me.phase_init == 0 then
            me.timer = WAIT
            place(me)
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t < 0 then
            dimming_chips.done(me)
        end
    end

    local kind = define.kind {
        id = id,
        pool = "effect",
        state = { bonus = "u16", variant = "record:countdown-variant" },
        -- `sub_80E31D8`.
        update = dimming_chips.phases { name = "TimeBom", effect = effect },
    }

    -- A chip's `dimming` hook (`sub_80E3242`): the controller for `user`,
    -- setting a bomb of `variant`.
    local function hook(variant: CountdownVariant): (user: Object, spec: DimmingChipSpec) -> Object?
        return function(user: Object, spec: DimmingChipSpec): Object?
            local o = dimming_chips.spawn(kind, user, spec)
            if o then
                local s = o.state :: State
                s.bonus = spec.bonus
                s.variant = variant
            end
            return o
        end
    end

    return { kind = kind, hook = hook }
end

return { make = make }
