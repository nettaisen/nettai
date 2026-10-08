--!strict
-- SumnBlk's dimming controller (the original's effect object #0x7C,
-- `sub_80E9120`; docs/engine/dimming-chips.md §9.1). The usual dimming
-- phases (@exelib/dimming), with an effect (`sub_80E9164`): with a hole in
-- front of the user and nothing on it, the navi (navi.luau) comes out of
-- it, and the effect waits for it to be gone; else it is over at once.

local dimming_chips = require("@exelib/dimming")
local navi = require("./navi")

local controller = {}

export type State = {
    -- The navi's palette (EV+0: the original keeps the chip's Param1 there; the palette is three times it).
    palette: number,
    -- The navi is out (EV+4, which the navi clears).
    busy: boolean,
    -- The telop's bonus (+0x32), which goes into the damage.
    bonus: number,
}

-- `dword_80E91E0`: a solid panel, or a body on it.
local NOT_A_FREE_HOLE = 0x0F80_0010

-- `sub_80E9164`.
local function effect(me: Object)
    local s = me.state :: State
    if me.phase_init ~= 0 then
        if not s.busy then
            dimming_chips.done(me)
        end
        return
    end
    local x, y = me.panel_x + me.facing, me.panel_y
    if bit32.band(field.flags(x, y), NOT_A_FREE_HOLE) ~= 0 then
        dimming_chips.done(me)
        return
    end
    local word = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    navi.spawn(me, x, y, me.element, s.palette, int.u32(word + s.bonus))
    me.phase_init = 4
end

controller.kind = new.kind {
    id = "sumnblk/controller",
    pool = "effect",
    state = { palette = "u8", busy = "bool", bonus = "u16" },
    update = dimming_chips.phases { name = "SumnBlk", effect = effect },
}

-- `sub_80E91B8`: a SumnBlk chip's `dimming` hook, its navi in its `palette`.
function controller.hook(palette: number): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(controller.kind, user, spec)
        if o then
            local s = o.state :: State
            s.palette, s.bonus = palette, spec.bonus
        end
        return o
    end
end

return controller
