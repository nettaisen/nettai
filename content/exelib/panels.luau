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

-- `object_getClosestPanelMatchingRowFiltered` (EXE6's, EXE5's, EXE4's
-- 0x0800A1AE): in row `y`, from the far edge of `side`'s field (column 6
-- for side 0, 1 for side 1) back toward `side`'s, the first panel whose
-- flags meet (require, forbid); 0 when it walks off the field.
function panels.closest_in_row(side: number, y: number, require: number, forbid: number): number
    local x = 6 - 5 * side
    local step = 2 * side - 1
    while true do
        if field.check(x, y, require, forbid) then
            return x
        end
        x += step
        if not field.valid(x, y) then
            return 0
        end
    end
end

-- `sub_800D190` (EXE4's 0x0800A1E4): across the rows, the front-most of
-- `side`'s panels whose flags meet (require, forbid): each row's
-- `closest_in_row`, the greatest of them for side 0, the least for side 1;
-- 0 without one.
function panels.front_most(side: number, require: number, forbid: number): number
    local found: { number } = {}
    for y = 1, 3 do
        local x = panels.closest_in_row(side, y, require, forbid)
        if x ~= 0 then
            table.insert(found, x)
        end
    end
    if #found == 0 then
        return 0
    end
    local x = found[1]
    for i = 2, #found do
        if side == 0 then
            x = math.max(x, found[i])
        else
            x = math.min(x, found[i])
        end
    end
    return x
end

-- `sub_800D618`: (x, y) is `victim`'s home but someone else holds it.
function panels.taken_from(x: number, y: number, victim: number): boolean
    local p = field.panel(x, y)
    return p ~= nil and p.home ~= p.alliance and p.home == victim
end

-- `sub_800D6CC`: column x is `victim`'s home, and all its.
function panels.full_home_column(x: number, victim: number): boolean
    if field.column(x).home ~= victim then
        return false
    end
    for y = 1, 3 do
        local p = field.panel(x, y)
        if not p or p.alliance ~= victim then
            return false
        end
    end
    return true
end

-- `sub_800D668`: `victim` may lose (x, y): its column isn't `victim`'s
-- home, or `victim` keeps another full home column on either side before
-- a panel it already lost.
function panels.can_take(x: number, y: number, victim: number): boolean
    if field.column(x).home ~= victim then
        return true
    end
    for _, dir in { 1, -1 } do
        local c = x + dir
        while field.valid(c, y) and not panels.taken_from(c, y, victim) do
            if panels.full_home_column(c, victim) then
                return true
            end
            c += dir
        end
    end
    return false
end

-- `sub_800D4D0`: the front column of `side`'s area in row `y`, as `side`
-- faces with `flip`: from the far side's back column (`sub_800D53C`: from
-- column 6 or 1, toward `side`'s, past the panels `side` holds; 0 off the
-- field), on to the first panel of `side`'s (`byte_800D52C`: the owner
-- bit 0x20 for side 1, none for side 0), and one back. 0 for none; the
-- column one back isn't checked to be on the field.
function panels.area_front(side: number, flip: number, y: number): number
    -- `object_getFlipDirection`: +1, or -1 for side ^ flip.
    local dir = 1 - 2 * bit32.bxor(side, flip)
    local x = 6 - 5 * bit32.bxor(side, flip)
    while true do
        if not field.valid(x, y) then
            return 0
        end
        local p = field.panel(x, y) :: Panel
        if p.alliance ~= side then
            break
        end
        x -= dir
    end
    local require, forbid = if side == 1 then 0x20 else 0, if side == 1 then 0 else 0x20
    while not field.check(x, y, require, forbid) do
        x -= dir
        if not field.valid(x, y) then
            return 0
        end
    end
    return x + dir
end

-- `GetRandomRelativePanelFiltered`: of region `region`'s panels around
-- (x, y), dx toward `dir`, one with every `require` bit and no `forbid`
-- bit, at random (one draw of the RNG, only when there is one); nil if
-- none (a whole-field region has no panels around a point).
function panels.random_in_region(
    x: number,
    y: number,
    require: number,
    forbid: number,
    region: Region,
    dir: number
): (number?, number?)
    local shape: { { number } } = region.panels or {}
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
function panels.random_enemy_panel(me: Object, x: number, y: number, region: Region): (number?, number?)
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
local function region_shape(n: Region): { { number } }
    local shape = n.panels
    if not shape then
        error("a whole-field region has no panel offsets")
    end
    return shape
end

-- `object_highlightPanelRegion`: highlight region `n`'s panels around (x,
-- y), dx along `facing` (+1 right, -1 left).
function panels.highlight_region(x: number, y: number, n: Region, facing: number)
    for _, o in region_shape(n) do
        field.highlight(x + o[1] * facing, y + o[2])
    end
end

-- `object_getPanelRegion`: region `n`'s panels around (x, y), dx along
-- `direction`, that pass `field.check`.
function panels.region(
    x: number,
    y: number,
    n: Region,
    direction: number,
    require: number,
    forbid: number
): { number }
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
function panels.crack_region(x: number, y: number, n: Region)
    for _, o in region_shape(n) do
        field.crack(x + o[1], y + o[2])
    end
end

return panels
