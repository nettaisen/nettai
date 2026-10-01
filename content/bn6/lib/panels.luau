--!strict
-- Panel lists and searches the chips' objects share. A panel in a list is
-- packed as the game packs it, x | y << 4 (so a list fits a `u8[18]` state
-- field); `panels.x` and `panels.y` unpack it.

local panels = {}

function panels.pack(x: number, y: number): number
    return x + y * 0x10
end

function panels.x(p: number): number
    return bit32.band(p, 7)
end

function panels.y(p: number): number
    return bit32.rshift(p, 4)
end

-- `object_getPanelsFiltered`: every panel whose flags meet (require,
-- forbid), from (6, 3) back to (1, 1): rows 3 to 1, columns 6 to 1.
function panels.filtered(require: number, forbid: number): { number }
    local list = {}
    for y = 3, 1, -1 do
        for x = 6, 1, -1 do
            if field.check(x, y, require, forbid) then
                table.insert(list, panels.pack(x, y))
            end
        end
    end
    return list
end

-- `object_getPanelsExceptCurrentFiltered`: the same, without `me`'s panel.
function panels.filtered_except(me: Object, require: number, forbid: number): { number }
    local list = {}
    for y = 3, 1, -1 do
        for x = 6, 1, -1 do
            if field.check(x, y, require, forbid) and not (x == me.panel_x and y == me.panel_y) then
                table.insert(list, panels.pack(x, y))
            end
        end
    end
    return list
end

-- `sub_8000C72`: shuffle the first `n` entries of `list` in place with
-- `count` swaps of two entries picked at random (two `rng_positive() % n`
-- draws each). (With no entries the game divides by zero; with no swaps
-- it loops about 2^32 times.)
function panels.shuffle(list: { number }, n: number, count: number)
    if n == 0 then
        error("sub_8000C72 shuffles an empty list (it divides by zero)")
    end
    if count == 0 then
        error("sub_8000C72 with no swaps loops 2^32 times")
    end
    for _ = 1, count do
        local i = battle.rng_positive() % n + 1
        local j = battle.rng_positive() % n + 1
        list[i], list[j] = list[j], list[i]
    end
end

-- `object_getAllianceDirection`: +1 for side 0, -1 for side 1 (the side's
-- forward, whatever the object faces).
-- The game doesn't mask its argument: a caller that passes a side and flip
-- halfword (objects/seed) gets a step far off the field.
function panels.side_direction(alliance: number): number
    return -(alliance * 2 - 1)
end

-- `GetRandomRelativePanelFiltered`: of region `region`'s panels around
-- (x, y), dx toward `dir`, one with every `require` bit and no `forbid`
-- bit, at random (one draw of the RNG, only when there is one); nil if
-- none. (Deprecated: a region by number.)
function panels.random_in_region(
    x: number,
    y: number,
    require: number,
    forbid: number,
    region: Region | number,
    dir: number
): (number?, number?)
    local shape: { { number } }
    if type(region) == "number" then
        local numbered = data.regions[region]
        if numbered == nil then
            error(string.format("GetRandomRelativePanelFiltered: region %d is not in the content", region))
        end
        shape = numbered
    else
        shape = region.panels or {}
    end
    local found: { number } = {}
    for _, p in shape do
        local px, py = x + p[1] * dir, y + p[2]
        if field.check(px, py, require, forbid) then
            -- The game keeps them as bytes: x in the low 3 bits, y above.
            table.insert(found, bit32.bor(bit32.band(px, 0xFF), bit32.lshift(bit32.band(py, 0xFF), 4)))
        end
    end
    if #found == 0 then
        return nil, nil
    end
    local pick = found[battle.rng_positive() % #found + 1]
    return bit32.band(pick, 7), bit32.band(bit32.rshift(pick, 4), 0xF)
end

-- `GetRandomRelativePanelFiltered` with a region definition: of `r`'s
-- panels around (x, y), dx toward `dir`, one with every `require` bit and
-- no `forbid` bit, at random (one draw of the RNG, only when there is
-- one); nil if none.
function panels.random_in(x: number, y: number, require: number, forbid: number, r: Region, dir: number): (number?, number?)
    local shape = r.panels
    if shape == nil then
        error("GetRandomRelativePanelFiltered takes a region of panels, not a whole-field one")
    end
    local found: { number } = {}
    for _, p in shape do
        local px, py = x + p[1] * dir, y + p[2]
        if field.check(px, py, require, forbid) then
            -- The game keeps them as bytes: x in the low 3 bits, y above.
            table.insert(found, bit32.bor(bit32.band(px, 0xFF), bit32.lshift(bit32.band(py, 0xFF), 4)))
        end
    end
    if #found == 0 then
        return nil, nil
    end
    local pick = found[battle.rng_positive() % #found + 1]
    return bit32.band(pick, 7), bit32.band(bit32.rshift(pick, 4), 0xF)
end

-- `off_8109784`: an enemy of each side stands there (the other side's
-- body bit).
local ENEMY_BODY = { [0] = 0x0400_0000, 0x0800_0000 }

-- `sub_8109708`: a random panel of region `region` around (x, y), forward
-- for `me`'s side, that an enemy of `me` stands on; nil if none.
function panels.random_enemy_panel(me: Object, x: number, y: number, region: Region | number): (number?, number?)
    return panels.random_in_region(x, y, ENEMY_BODY[me.alliance], 0, region, panels.side_direction(me.alliance))
end

-- `object_getPanelsInColumnFiltered`: the panels of column x (y 1 to 3)
-- that pass `field.check`.
function panels.in_column(x: number, require: number, forbid: number): { number }
    local list = {}
    for y = 1, 3 do
        if field.check(x, y, require, forbid) then
            table.insert(list, panels.pack(x, y))
        end
    end
    return list
end

-- An enemy's body on a panel, while dimmed too, by `me`'s side
-- (`byte_80ED080`).
local ENEMY_COLUMN = { [0] = 0x0401_0000, [1] = 0x0801_0000 }

-- `sub_80ED040` (MachGun's, the dragons'): from the column ahead of `me`
-- forward, the first with an enemy's body in some row, and true; with
-- none, the last column ahead, and false.
function panels.enemy_column(me: Object): (number, boolean)
    local dir, x = me.facing, me.panel_x
    local enemy = ENEMY_COLUMN[me.alliance]
    while true do
        x += dir
        if x == 0 or x >= 7 then
            break
        end
        if x < 0 then
            error("sub_80ED040 searches off the field's left edge (it would run on for 2^31 columns)")
        end
        for y = 1, 3 do
            if field.check(x, y, enemy, 0) then
                return x, true
            end
        end
    end
    return if dir > 0 then 6 else 1, false
end

-- A hit region's offsets ({dx, dy} each, dx toward the facing side).
local function region_shape(n: number): { { number } }
    local r = data.regions[n]
    if not r then
        error(string.format("panel region %#x reads past PanelOffsetListsPointerTable", n))
    end
    return r
end

-- `object_highlightPanelRegion`: highlight region `n`'s panels around (x,
-- y), dx along `facing` (+1 right, -1 left).
function panels.highlight_region(x: number, y: number, n: number, facing: number)
    for _, o in region_shape(n) do
        field.highlight(x + o[1] * facing, y + o[2])
    end
end

-- `object_getPanelRegion`: region `n`'s panels around (x, y), dx along
-- `direction`, that pass `field.check`.
function panels.region(x: number, y: number, n: number, direction: number, require: number, forbid: number): { number }
    local list = {}
    for _, o in region_shape(n) do
        local px, py = x + o[1] * direction, y + o[2]
        if field.check(px, py, require, forbid) then
            table.insert(list, panels.pack(px, py))
        end
    end
    return list
end

-- `sub_80DB48A`: crack (`object_crackPanel`) each panel of region `n`
-- around (x, y), its offsets as they are (not turned to a side).
function panels.crack_region(x: number, y: number, n: number)
    for _, o in region_shape(n) do
        field.crack(x + o[1], y + o[2])
    end
end

return panels
