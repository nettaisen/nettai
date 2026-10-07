--!strict
-- `ho_8026554`: the panel a navi attacks a target from in a Beast Out lock-on mode (./lockon's records), which the
-- rush (./beast/rush) and the Beast claw's, the lunge's and GroundCross's drill's setups ask (./lockon's `panel`),
-- with what the modes share (`Search`: ./lockon's `search`).

-- The column shifts toward the user tried, in order, when no panel next to the target fits (`byte_8026735`); what
-- every panel between the chosen one and the target must be for the modes that need a clear path, by alliance
-- (`byte_8026544`).
export type Search = { column_shifts: { number }, clear_path: { PanelCondition } }

local lockon_search = {}

-- `sub_8026510`: every panel from (x, y) toward the front, up to the target's column, meets the clear-path
-- condition (a panel off the field never does).
local function path_clear(navi: Object, x: number, y: number, target_x: number, search: Search): boolean
    local front = navi.facing
    local cond = search.clear_path[bit32.band(navi.alliance, 1) + 1]
    while true do
        if x < 0 or x > 0xFF or not field.check(x, y, cond.require, cond.forbid) then
            return false
        end
        x += front
        if x == target_x then
            return true
        end
    end
end

-- `sub_8026450` (`sub_80264A8` with `clear_path`): the first panel of `offsets` from (x, y) the navi can stand on
-- (`sub_800E680`), on the field's columns, not past the target's column, and with a clear path to it if asked.
local function search_from(
    navi: Object,
    target_x: number,
    x: number,
    y: number,
    offsets: { { number } },
    clear_path: boolean,
    search: Search
): (number?, number?)
    local front = navi.facing
    for _, off in offsets do
        local cx, cy = x + front * off[1], y + off[2]
        if
            cx >= 0
            and cx <= 6
            and cx * front <= target_x * front
            and cy >= 0
            and cy <= 3
            and navi:can_stand_any_side(cx, cy)
            and (not clear_path or path_clear(navi, cx, cy, target_x, search))
        then
            return cx, cy
        end
    end
    return nil, nil
end

-- `sub_8026622`: along the target's row from the navi's own column (the same-row list when they share a row), then
-- along the rows the mode's shifts away from the target's, with the plain list.
local function search_row(navi: Object, tx: number, ty: number, mode: Lockon, search: Search): (number?, number?)
    local own_x = navi.panel_x
    local offsets = mode.offsets or {}
    local first = if ty == navi.panel_y then mode.same_row_offsets or {} else offsets
    local x, y = search_from(navi, tx, own_x, ty, first, false, search)
    if x ~= nil then
        return x, y
    end
    for _, shift in mode.row_shifts or {} do
        x, y = search_from(navi, tx, own_x, ty + shift, offsets, false, search)
        if x ~= nil then
            return x, y
        end
    end
    return nil, nil
end

-- `sub_80265D0` (with the column shifts) or `sub_8026450` / `sub_80264A8` alone: next to the target, from the
-- far-column list when the target stands in the column farthest ahead (`sub_80266BA`: column 6 facing right, 1
-- facing left).
local function search_near(navi: Object, tx: number, ty: number, mode: Lockon, search: Search): (number?, number?)
    local front = navi.facing
    local far_column = if front > 0 then 6 else 1
    local offsets = mode.offsets or {}
    if mode.far_column_offsets ~= nil and tx == far_column then
        offsets = mode.far_column_offsets
    end
    local clear_path = mode.clear_path == true
    local x, y = search_from(navi, tx, tx, ty, offsets, clear_path, search)
    if x ~= nil or not mode.column_shifts then
        return x, y
    end
    for _, shift in search.column_shifts do
        x, y = search_from(navi, tx, tx + front * shift, ty, offsets, clear_path, search)
        if x ~= nil then
            return x, y
        end
    end
    return nil, nil
end

-- `ho_8026554`: the panel to attack (tx, ty) from in lock-on `mode`: no mode (the original's mode 0), or a target off
-- the field's playable panels, is the navi's own panel; (0, 0x7F) when no panel fits (`sub_80265D0`'s registers
-- then).
function lockon_search.panel(navi: Object, tx: number, ty: number, mode: Lockon?, search: Search): (number, number)
    if mode == nil or not field.valid(tx, ty) or mode.rule == "stay" then
        -- (`sub_802661C`.)
        return navi.panel_x, navi.panel_y
    end
    local x, y
    if mode.rule == "row" then
        x, y = search_row(navi, tx, ty, mode, search)
    else
        x, y = search_near(navi, tx, ty, mode, search)
    end
    if mode.prefers_middle_row then
        -- `sub_80265FE`: the middle row of that column, if it will do. (Its not-found test reads flags a `mov` just
        -- set, so it always runs.)
        local mx = x or 0
        if navi:can_stand_any_side(mx, 2) then
            return mx, 2
        end
    end
    if x ~= nil and y ~= nil then
        return x, y
    end
    return 0, 0x7F
end

return lockon_search
