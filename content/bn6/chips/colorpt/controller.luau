--!strict
-- ColorPt's and DblPoint's dimming controller (the original's effect
-- object #0x50, `sub_80E6480`; docs/engine/dimming-chips.md §10.1). The
-- usual dimming phases (lib/dimming), with an effect (`sub_80E64C4`) that
-- sends a point (point.luau) from each panel of the user's front column
-- that is solid, free and may go to the other side (`sub_80E650A`), then
-- waits until they are all gone.

local dimming_chips = require("../../lib/dimming")
local panels = require("../../lib/panels")
local point = require("./point")

local controller = {}

export type State = {
    point: ColorPoint,
    -- The points still out, by the order they went (EV+0, a byte each).
    out: { boolean },
}

-- The panel a point leaves from: solid, with nothing on it.
local SOLID, OCCUPIED = 0x10, 0x0F88_0080

-- `sub_80E650A`: a point from each row's front panel that qualifies.
local function send_points(me: Object)
    local s = me.state :: State
    for k = 1, 3 do
        s.out[k] = false
    end
    local slot = 1
    for y = 1, 3 do
        local x = panels.area_front(bit32.bxor(me.alliance, 1), me.flip, y)
        if x ~= 0 and field.check(x, y, SOLID, OCCUPIED) and panels.can_take(x, y, me.alliance) then
            point.spawn(me, x, y, s.point, slot)
            slot += 1
        end
    end
end

-- `sub_80E64C4`.
local function effect(me: Object)
    if me.phase_init == 0 then
        send_points(me)
        me.timer = 0x1E
        me.phase_init = 4
    end
    local s = me.state :: State
    for k = 1, 3 do
        if s.out[k] then
            return
        end
    end
    dimming_chips.done(me)
end

controller.kind = define.kind {
    id = "bn6:colorpt/controller",
    pool = "effect",
    state = { point = "record:colorpt-point", out = "bool[3]" },
    update = dimming_chips.phases { name = "ColorPt", effect = effect },
}

-- `sub_80E64E8`: a ColorPt chip's `dimming` hook, for its points.
function controller.hook(spec: ColorPoint): (user: Object, spec: DimmingChipSpec) -> Object?
    local p = define.record("colorpt-point", spec)
    return function(user: Object, chip: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(controller.kind, user, chip)
        if o then
            (o.state :: State).point = p
        end
        return o
    end
end

return controller
