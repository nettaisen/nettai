--!strict
-- AreaGrab's and PanelGrab's dimming controller, as EXE5 and EXE6 share it
-- (EXE6's effect object #3, `sub_80E0710`; EXE5's #3): each game makes its
-- own (`grab.make`: its kind and its grab shot's). The usual dimming
-- phases (@exelib/dimming), with an effect that drops grab shots
-- (@exelib/grab/shot) on the first column in front
-- not wholly the user's side's (AreaGrab) or on the first panel past the
-- user's side's front in the user's row (PanelGrab), then waits 61 ticks.
-- See docs/engine/chips.md §3.6.8. Each chip's `dimming` hook is the made
-- controller's `area` or `panel` (`sub_80E07E0`, `off_802CCB4[0]`).

local dimming_chips = require("../dimming")
local shot = require("./shot")

local grab = {}

export type State = {
    -- The Atk+ / cross bonus the telop shows (object +0x32); it adds to
    -- the grab shots' damage.
    bonus: number,
    -- AreaGrab's whole column (the original's first chip parameter, 1),
    -- or PanelGrab's one panel (0).
    whole_column: boolean,
}

-- `sub_800D5F0`: how many panels of column `x` aren't `side`'s.
local function foreign_panels(x: number, side: number): number
    local n = 0
    for y = 1, 3 do
        local p = field.panel(x, y)
        if p and p.alliance ~= side then
            n += 1
        end
    end
    return n
end

-- `sub_800D5BA`: from the user's column back, the nearest column wholly
-- its side's; if there's none behind, forward from its column.
local function home_column(me: Object): number
    local side, front = me.alliance, me.facing
    local x, step = me.panel_x, -front
    local restarts = 0
    while true do
        if not field.valid(x, 1) then
            -- Off the field: start over from the user's column, forward.
            -- (Forward off the field too, the game loops here forever.)
            restarts += 1
            if restarts > 1 then
                error("AreaGrab found no column wholly its side's: sub_800D5BA loops forever here")
            end
            x, step = me.panel_x, front
        elseif foreign_panels(x, side) ~= 0 then
            x += step
        else
            return x
        end
    end
end

-- `sub_800D58C`: from there, forward to the first column with a panel
-- that isn't the user's side's (nil past the field).
local function column_to_grab(me: Object): number?
    local side, front = me.alliance, me.facing
    local x = home_column(me)
    while foreign_panels(x, side) == 0 do
        x += front
        if not field.valid(x, 1) then
            return nil
        end
    end
    return x
end

-- The panel filter that finds the user's side's panels (`byte_800D52C`,
-- by side): side 1's have the owner bit (0x20), side 0's don't.
local OWN_SIDE = { [0] = { require = 0, forbid = 0x20 }, [1] = { require = 0x20, forbid = 0 } }

-- `object_getEdgePanelMatchingRow` (`sub_800D4D0`): in the controller's
-- row, the first panel past the user's side's front (nil if none).
local function edge_panel(me: Object): (number?, number?)
    local side, y, front = me.alliance, me.panel_y, me.facing
    -- sub_800D53C: from the far edge, back while the panels are the
    -- user's side's.
    local x = 6 - 5 * bit32.bxor(me.alliance, me.flip)
    while true do
        if not field.valid(x, y) then
            return nil, nil
        end
        local p = field.panel(x, y) :: Panel
        if p.alliance ~= side then
            break
        end
        x -= front
    end
    -- object_getFirstPanelInDirectionFiltered: on back to the user's
    -- side's first panel.
    local own = OWN_SIDE[side]
    while not field.check(x, y, own.require, own.forbid) do
        x -= front
        if not field.valid(x, y) then
            return nil, nil
        end
    end
    x += front
    if field.valid(x, y) then
        return x, y
    end
    return nil, nil
end

-- The ticks the controller waits after dropping the shots (61: the count
-- runs out below 0).
local WAIT = 0x3C

-- `sub_80E0754`: the grab shots (of `shot_kind`, running while dimmed),
-- then 61 ticks.
local function effect(me: Object, shot_kind: Kind)
    local s = me.state :: State
    if me.phase_init == 0 then
        local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16)) + s.bonus
        local function drop(x: number, y: number)
            local o = shot.spawn(shot_kind, me, x, y, me.element, damage)
            if o then
                o.run_while_dimmed = true
            end
        end
        if s.whole_column then
            local x = column_to_grab(me)
            if x then
                for y = 1, 3 do
                    drop(x, y)
                end
            end
        else
            local x, y = edge_panel(me)
            if x and y then
                drop(x, y)
            end
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

-- A game's controller: its kind's key, and its grab shot's kind.
export type Spec = { id: string, shot: Kind }

export type Grab = {
    kind: Kind,
    -- AreaGrab's hook: the first column in front not wholly the user's
    -- side's.
    area: (user: Object, spec: DimmingChipSpec) -> Object?,
    -- PanelGrab's: the first panel past the user's side's front, in its
    -- row.
    panel: (user: Object, spec: DimmingChipSpec) -> Object?,
}

-- A game's controller.
function grab.make(spec: Spec): Grab
    local kind = define.kind {
        id = spec.id,
        pool = "effect",
        state = { bonus = "u16", whole_column = "bool" },
        update = dimming_chips.phases {
            name = "AreaGrab",
            effect = function(me: Object)
                effect(me, spec.shot)
            end,
        },
    }

    -- A chip's `dimming` hook (`sub_80E07E0`): the controller for `user`.
    local function hook(whole_column: boolean): (user: Object, spec: DimmingChipSpec) -> Object?
        return function(user: Object, chip: DimmingChipSpec): Object?
            local o = dimming_chips.spawn(kind, user, chip)
            if o then
                local s = o.state :: State
                s.bonus = chip.bonus
                s.whole_column = whole_column
            end
            return o
        end
    end

    return { kind = kind, area = hook(true), panel = hook(false) }
end

return grab
