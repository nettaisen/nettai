--!strict
-- The hammer chips' dimming controller (the dimming chips' variant 0x10: effect object #0x26, 0x080E4B88; its effect
-- 0x080E4BCC: EXE4's own): the usual dimming phases (@exelib/dimming), with an effect that sets the chip's hammer
-- (./hammer) on the panel in front of the user if it is solid and free, of the chip's element and its damage with the
-- telop's bonus, then waits 31 ticks. Each chip makes its own (`controller.make` with its hammer); its spawn
-- (0x080E4C16) takes the user's side, not its facing.

local dimming_chips = require("@exelib/dimming")
local panels = require("@exelib/panels")

local controller = {}

export type State = {
    -- The bonus the telop shows (the original's +0x32), added to the hammer's damage.
    bonus: number,
}

local WAIT = 30
-- 0x080E4BE0: the panel in front must be solid and free.
local SOLID, OCCUPIED = 0x10, 0x0F88_0080

-- A chip's controller (kind `id`) setting its hammer (`spawn`, ./hammer's): the chip's `dimming` hook.
function controller.make(
    id: string,
    spawn: (owner: Object, x: number, y: number, element: number, damage: number) -> Object?
): (user: Object, spec: DimmingChipSpec) -> Object?
    -- 0x080E4BCC.
    local function effect(me: Object)
        if me.phase_init == 0 then
            local x, y = me.panel_x + panels.side_direction(me.alliance), me.panel_y
            if field.check(x, y, SOLID, OCCUPIED) then
                spawn(me, x, y, me.element, dimming_chips.damage_plus(me, (me.state :: State).bonus))
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
        id = id,
        pool = "effect",
        state = { bonus = "u16" },
        -- 0x080E4B88.
        update = dimming_chips.phases { name = "the hammer chips", effect = effect },
    }

    -- 0x080E4C16.
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(kind, user, spec, true)
        if o then
            (o.state :: State).bonus = spec.bonus
        end
        return o
    end
end

return controller
