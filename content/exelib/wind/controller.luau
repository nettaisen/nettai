--!strict
-- Wind's and Fan's dimming controller (effect object #0x25, EXE6's `sub_80E309C`, EXE5's 0x080E329C: the same code;
-- docs/engine/dimming-chip-effects.md §1): the usual dimming phases, with an effect that places the chip's fan on the
-- panel in front of the user if it is free solid ground, and waits 31 ticks. A maker taking a game's fan and where
-- the controller spawns.

local dimming_chips = require("@exelib/dimming")

local controller = {}

export type State = { variant: FanVariant }

local WAIT = 0x1E
-- Where the fan goes: a solid panel with nothing on it.
local SOLID, OCCUPIED = 0x10, 0x0F88_0080

function controller.make(look: WindControllerLook): WindController
    -- `sub_80E30E0`: the fan, then 31 ticks.
    local function effect(me: Object)
        if me.phase_init == 0 then
            local x, y = me.panel_x + me.facing, me.panel_y
            if field.check(x, y, SOLID, OCCUPIED) then
                local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
                look.spawn_fan(me, (me.state :: State).variant, x, y, damage)
            end
            me.timer = WAIT
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t < 0 then
            dimming_chips.done(me)
        end
    end

    local kind = new.kind {
        id = look.id,
        pool = "effect",
        state = { variant = "record:fan-variant" },
        -- `sub_80E309C`.
        update = dimming_chips.phases { name = "Wind's controller", effect = effect },
    }

    local function hook(variant: FanVariant): (user: Object, spec: DimmingChipSpec) -> Object?
        return function(user: Object, spec: DimmingChipSpec): Object?
            local o = dimming_chips.spawn(kind, user, spec)
            if o then
                if look.position then
                    o.pos = look.position(user, spec)
                end
                (o.state :: State).variant = variant
            end
            return o
        end
    end

    return { kind = kind, hook = hook }
end

return controller
