--!strict
-- EXE4's AreaGrab and PanelGrab: their grab shot (attack object #0xF, 0x080CEFD4: @exelib/grab/shot, EXE5's code but
-- that it spares the other side's back column, 0x080CF0A6), EXE4's look (its sheet, 0c-13, and sounds, 0x93 falling
-- and 0x94 landing) and collision types, and their dimming controller (effect object #3, 0x080E1F64:
-- @exelib/grab/controller), EXE4's kinds, with EXE4's own finders (0x080E1FA8): AreaGrab's column is the other side's
-- front-most panel across the rows (0x0800A1E4), PanelGrab's panel the one past the user's side's front in its row
-- (0x0800A1AE). Each chip's `dimming` hook is `grab.area` or `grab.panel`.

local collision = require("../rules/collision")
local controller = require("@exelib/grab/controller")
local shot = require("@exelib/grab/shot")

local shot_kind = shot.make_kind {
    id = "grab/shot",
    sprite = asset.sprite("grab-shot"),
    fall_sound = asset.sound("grab-shot-2"),
    land_sound = asset.sound("grab-shot"),
    target = collision.hits_navis,
    self_type = collision.thrown,
    spares = "back_column",
}

-- The panel filters by side (0x080E205C's: a side's panels have the owner bit 0x20 set for side 1, clear for side 0).
local OWN = { [0] = { require = 0, forbid = 0x20 }, [1] = { require = 0x20, forbid = 0 } }

-- 0x0800A1AE (`object_getClosestPanelMatchingRowFiltered`): in row `y`, from the far edge of `side`'s field (column 6
-- for side 0, 1 for side 1) back toward `side`'s, the first panel of `side`'s; 0 when it walks off the field.
local function closest_of_side(side: number, y: number): number
    local back = -(1 - 2 * side)
    local x = 6 - 5 * side
    while true do
        local own = OWN[side]
        if field.check(x, y, own.require, own.forbid) then
            return x
        end
        x += back
        if not field.valid(x, y) then
            return 0
        end
    end
end

-- 0x0800A1E4: AreaGrab's column, the other side's panels' front-most across the rows (each row's closest of that side
-- from its far edge): the least of them for a user of side 0, the greatest for side 1; none without one.
local function column(me: Object): number?
    local other = bit32.bxor(me.alliance, 1)
    local found: { number } = {}
    for y = 1, 3 do
        local x = closest_of_side(other, y)
        if x ~= 0 then
            table.insert(found, x)
        end
    end
    if #found == 0 then
        return nil
    end
    local x = found[1]
    for i = 2, #found do
        if other == 0 then
            x = math.max(x, found[i])
        else
            x = math.min(x, found[i])
        end
    end
    return x
end

-- 0x080E1FB6: PanelGrab's panel, the one past the user's side's front in the user's row, if it is on the field.
local function panel(me: Object): (number?, number?)
    local y = me.panel_y
    local x = closest_of_side(me.alliance, y)
    if x == 0 then
        return nil, nil
    end
    x += me.facing
    if not field.valid(x, y) then
        return nil, nil
    end
    return x, y
end

return controller.make { id = "grab/controller", shot = shot_kind, column = column, panel = panel }
