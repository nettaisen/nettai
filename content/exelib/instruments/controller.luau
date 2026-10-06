--!strict
-- The instrument chips' dimming controller (the original's effect object
-- #0x4A, `sub_80E5E00`; docs/engine/dimming-chips.md §6.1): Fanfare,
-- Discord, Timpani and Silence. The usual dimming phases (@exelib/dimming),
-- with an effect (`sub_80E5E44`) that sets the chip's instrument
-- (instrument.luau) on the panel in front of the user if it is solid and
-- free, then waits 60 ticks.
--
-- A chip's `dimming` hook is `instruments.hook(instrument)`; another
-- game's controller sets its own instruments (`instruments.make`).

local dimming_chips = require("../dimming")
local instrument = require("./instrument")

local instruments = {}

export type State = { instrument: Instrument }

local WAIT = 0x3C
-- `byte_80E5E98`: the panel in front must be solid and free (both sides).
local SOLID, OCCUPIED = 0x10, 0x0F88_0080

-- A game's controller (kind `id`) setting instruments of
-- `instrument_kind`: its chips' hook of an instrument.
function instruments.make(id: string, instrument_kind: Kind): (Instrument) -> (user: Object, spec: DimmingChipSpec) -> Object?
    -- `sub_80E5E44`.
    local function effect(me: Object)
        if me.phase_init == 0 then
            local x, y = me.panel_x + me.facing, me.panel_y
            if field.check(x, y, SOLID, OCCUPIED) then
                local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
                instrument.spawn_kind(instrument_kind, me, x, y, (me.state :: State).instrument, damage)
            end
            me.timer = WAIT
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t <= 0 then
            dimming_chips.done(me)
        end
    end

    local kind = new.kind {
        id = id,
        pool = "effect",
        state = { instrument = "record:instrument" },
        update = dimming_chips.phases { name = "the instrument chips", effect = effect },
    }

    -- `sub_80E5EA8`: a chip's `dimming` hook, for its instrument.
    local function hook(i: Instrument): (user: Object, spec: DimmingChipSpec) -> Object?
        return function(user: Object, spec: DimmingChipSpec): Object?
            local o = dimming_chips.spawn(kind, user, spec)
            if o then
                local s = o.state :: State
                s.instrument = i
            end
            return o
        end
    end

    return hook
end

-- EXE6's.

return instruments
