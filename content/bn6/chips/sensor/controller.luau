--!strict
-- Sensor's dimming controller (the original's effect object #0x65,
-- `sub_80E7BFC`; docs/engine/dimming-chips.md §8.1). The usual dimming
-- phases (lib/dimming), with an effect (`sub_80E7C40`) that sets the
-- turret (turret.luau) on the panel in front of the user if it is free,
-- with the chip's damage and its Atk+ bonus, then waits 60 ticks.

local dimming_chips = require("../../lib/dimming")
local turret = require("./turret")

local controller = {}

-- A Sensor chip's turret: its palette and HP.
export type Turret = { palette: number, hp: number }

export type State = {
    palette: number,
    hp: number,
    -- The telop's bonus (+0x32), which goes into the damage.
    bonus: number,
}

local WAIT = 0x3C
-- `byte_80E7C98`: the panel in front must be free (both sides).
local OCCUPIED = 0x0F88_0080

-- `sub_80E7C40`.
local function effect(me: Object)
    if me.phase_init == 0 then
        local x, y = me.panel_x + me.facing, me.panel_y
        if field.check(x, y, 0, OCCUPIED) then
            local s = me.state :: State
            local word = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
            turret.spawn(me, x, y, me.element, int.u32(word + s.bonus), s.palette, s.hp)
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

controller.kind = define.kind {
    id = "bn6:sensor/controller",
    pool = "effect",
    state = { palette = "u8", hp = "u8", bonus = "u16" },
    update = dimming_chips.phases { name = "Sensor", effect = effect },
}

-- `sub_80E7CA8`: a Sensor chip's `dimming` hook, for its turret.
function controller.hook(t: Turret): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(controller.kind, user, spec)
        if o then
            local s = o.state :: State
            s.palette, s.hp, s.bonus = t.palette, t.hp, spec.bonus
        end
        return o
    end
end

return controller
