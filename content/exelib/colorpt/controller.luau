--!strict
-- ColorPt's and DblPoint's dimming controller, as EXE5 and EXE6 share it (EXE6's effect object #0x50, `sub_80E6480`;
-- EXE5's #0x50, 0x080E650C, the same code; EXE6's docs/engine/dimming-chips.md §10.1). The usual dimming phases (EXE6's
-- lib/dimming), with an effect (`sub_80E64C4`) that sends a point (@exelib/colorpt/point) from each panel of the
-- user's front column that is solid, free and may go to the other side (`sub_80E650A`), then waits until they are
-- all gone. Each game makes its own (`controller.make`) with its point; a chip's `dimming` hook is the made
-- controller's `hook(point)`.

local dimming_chips = require("@exelib/dimming")
local panels = require("@exelib/panels")

local controller = {}

export type State = {
    point: ColorPoint,
    -- The points still out, by the order they went (EV+0, a byte each).
    out: { boolean },
}

export type Spec = {
    -- The kind's id.
    id: string,
    -- The game's point's spawn (@exelib/colorpt/point's).
    point: (holder: Object, x: number, y: number, spec: ColorPoint, slot: number) -> Object?,
}

-- The panel a point leaves from: solid, with nothing on it.
local SOLID, OCCUPIED = 0x10, 0x0F88_0080

function controller.make(spec: Spec): { kind: Kind, hook: (p: ColorPoint) -> (user: Object, chip: DimmingChipSpec) -> Object? }
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
                spec.point(me, x, y, s.point, slot)
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

    local kind = new.kind {
        id = spec.id,
        pool = "effect",
        state = { point = "record:colorpt-point", out = "bool[3]" },
        update = dimming_chips.phases { name = "ColorPt", effect = effect },
    }

    -- `sub_80E64E8`: a ColorPt chip's `dimming` hook, for its points.
    local function hook(p: ColorPoint): (user: Object, chip: DimmingChipSpec) -> Object?
        local rec = new.record("colorpt-point", p)
        return function(user: Object, chip: DimmingChipSpec): Object?
            local o = dimming_chips.spawn(kind, user, chip)
            if o then
                (o.state :: State).point = rec
            end
            return o
        end
    end

    return { kind = kind, hook = hook }
end

return controller
