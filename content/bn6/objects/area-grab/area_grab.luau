--!strict
-- The AreaGrab and PanelGrab chips' dimming controller (effect object #3,
-- `sub_80E0710`): the usual dimming phases (the dimming service), with an
-- effect that drops grab shots (objects/grab-shot) on the first column in
-- front not wholly the user's side's (AreaGrab: Param1 set) or on the
-- first panel past the user's side's front in the user's row (PanelGrab),
-- then waits 61 ticks. See docs/engine/chips.md §3.6.8.
--
-- The chips' script (both are dimming chip subtype 0): `dimming_chip` is
-- their controller's spawner, `off_802CCB4[0]`.

local grab_shot = require("../grab-shot/grab_shot")

export type State = {
    -- The chip and the Atk+ / cross bonus the telop shows (object +0x30,
    -- +0x32); the bonus adds to the grab shots' damage.
    chip: number,
    bonus: number,
}

local area_grab = {
    state = { chip = "u16", bonus = "u16" },
}

-- `sub_80E07E0`: the controller for `user`, on its panel, with the chip's
-- parameters and damage word. (Its position is the spawner's registers,
-- which nothing reads.)
function area_grab.dimming_chip(user: Object, spec: DimmingChipSpec): Object?
    local o = battle.spawn_kind("area-grab", Vec3.zero, spec.params)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = user.panel_x, user.panel_y
    o.element = spec.element
    o.related1 = user
    o.alliance, o.flip = user.alliance, user.flip
    o.damage = spec.damage
    o.stamina = bit32.rshift(spec.damage, 16)
    local s = o.state :: State
    s.chip, s.bonus = spec.chip, spec.bonus
    return o
end

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

-- `sub_80E0754`: the grab shots (running while dimmed), then 61 ticks.
local function effect(me: Object)
    if me.phase_init == 0 then
        local s = me.state :: State
        local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16)) + s.bonus
        local function drop(x: number, y: number)
            local shot = grab_shot.spawn(me, x, y, me.element, damage)
            if shot then
                shot.run_while_dimmed = true
            end
        end
        if me:param(1) == 0 then
            local x, y = edge_panel(me)
            if x and y then
                drop(x, y)
            end
        else
            local x = column_to_grab(me)
            if x then
                for y = 1, 3 do
                    drop(x, y)
                end
            end
        end
        me.timer = 0x3C
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        me:set_action(0xC)
    end
end

-- `sub_80E0710`: the dimming's phases around the effect.
function area_grab.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        dimming.begin(me)
    elseif l == "update" then
        local a = me.action
        if a == 0 then
            dimming.dim_screen(me)
        elseif a == 4 then
            dimming.show_telop(me)
        elseif a == 8 then
            effect(me)
        elseif a == 0xC then
            dimming.undim_screen(me)
        else
            error(string.format("AreaGrab's action %#x reads past its table", a))
        end
    else
        dimming.finish(me)
    end
end

return area_grab
