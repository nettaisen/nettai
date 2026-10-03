--!strict
-- AirRaid's dimming controller (the original's effect object #0x4B,
-- `sub_80E5ECC`; docs/engine/dimming-chips.md §7.1). The usual dimming
-- phases (@common/dimming), with an effect (`sub_80E5F10`) that sets the
-- plane (plane.luau) on the panel in front of the user if it is free,
-- with the chip's damage and its Atk+ bonus, then waits 60 ticks.

local dimming_chips = require("@common/dimming")
local plane = require("./plane")

local controller = {}

export type State = {
    plane: AirRaidPlane,
    -- The telop's bonus (+0x32), which goes into the damage.
    bonus: number,
}

local WAIT = 0x3C
-- `byte_80E5F68`: the panel in front must be free (both sides).
local OCCUPIED = 0x0F88_0080

-- `sub_80E5F10`.
local function effect(me: Object)
    if me.phase_init == 0 then
        local x, y = me.panel_x + me.facing, me.panel_y
        if field.check(x, y, 0, OCCUPIED) then
            local s = me.state :: State
            local word = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
            plane.spawn(me, x, y, s.plane, int.u32(word + s.bonus))
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
    id = "bn6:airraid/controller",
    pool = "effect",
    state = { plane = "record:airraid-plane", bonus = "u16" },
    update = dimming_chips.phases { name = "AirRaid", effect = effect },
}

-- `sub_80E5F78`: an AirRaid chip's `dimming` hook, for its plane.
function controller.hook(p: AirRaidPlane): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(controller.kind, user, spec)
        if o then
            local s = o.state :: State
            s.plane, s.bonus = p, spec.bonus
        end
        return o
    end
end

return controller
