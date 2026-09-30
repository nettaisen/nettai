--!strict
-- Panel searches the game's objects share (`object_getPanels*`). They
-- return panels as the game packs them, a byte each: y << 4 | x (which
-- `panels.unpack` splits), in the game's order.

local panels = {}

-- A packed panel's x and y.
function panels.unpack(p: number): (number, number)
    return bit32.band(p, 7), bit32.rshift(p, 4)
end

-- The flags word has every `require` bit and none of `forbid`'s (without
-- `object_checkPanelParameters`' check that the word is nonzero).
local function matches(x: number, y: number, require: number, forbid: number): boolean
    local f = field.flags(x, y)
    return bit32.band(f, forbid) == 0 and bit32.band(f, require) == require
end

-- `object_getPanelsFiltered`: every panel matching, from the back row and
-- column (y 3 to 1, x 6 to 1).
function panels.all(require: number, forbid: number): { number }
    local out = {}
    for y = 3, 1, -1 do
        for x = 6, 1, -1 do
            if matches(x, y, require, forbid) then
                table.insert(out, bit32.bor(bit32.lshift(y, 4), x))
            end
        end
    end
    return out
end

-- `object_getPanelsExceptCurrentFiltered`: the same, leaving out `me`'s
-- panel.
function panels.except(me: Object, require: number, forbid: number): { number }
    local out = {}
    for y = 3, 1, -1 do
        for x = 6, 1, -1 do
            if matches(x, y, require, forbid) and not (x == me.panel_x and y == me.panel_y) then
                table.insert(out, bit32.bor(bit32.lshift(y, 4), x))
            end
        end
    end
    return out
end

-- `object_getPanelsInColumnFiltered`: the panels of column x (y 1 to 3)
-- that pass `field.check`.
function panels.in_column(x: number, require: number, forbid: number): { number }
    local out = {}
    for y = 1, 3 do
        if field.check(x, y, require, forbid) then
            table.insert(out, bit32.bor(bit32.lshift(y, 4), x))
        end
    end
    return out
end

-- A panel region's offsets ({dx, dy} each, dx toward the facing side).
local function region(n: number): { { number } }
    local r = data.regions[n]
    if not r then
        error(string.format("panel region %#x reads past PanelOffsetListsPointerTable", n))
    end
    return r
end

-- `object_highlightPanelRegion`: highlight region `n`'s panels around (x,
-- y), dx along `facing` (+1 right, -1 left).
function panels.highlight_region(x: number, y: number, n: number, facing: number)
    for _, o in region(n) do
        field.highlight(x + o[1] * facing, y + o[2])
    end
end

-- `object_getPanelRegion`: region `n`'s panels around (x, y), dx along
-- `direction`, that pass `field.check`.
function panels.region(x: number, y: number, n: number, direction: number, require: number, forbid: number): { number }
    local out = {}
    for _, o in region(n) do
        local px, py = x + o[1] * direction, y + o[2]
        if field.check(px, py, require, forbid) then
            table.insert(out, bit32.bor(bit32.lshift(py, 4), px))
        end
    end
    return out
end

-- `sub_801BD3C`: effect `params` (its four parameters as one word: id,
-- flip, palette, priority) on each panel of region `n` around (x, y), dx
-- along side `side`'s direction, those on the field, at Z `z` (the
-- caller's r7, raw). A whole-field region (0x80 and up) puts it at Z 0 on
-- every panel meeting its condition, rows 3 to 1, columns 6 to 1.
function panels.effects_on_region(x: number, y: number, params: number, n: number, side: number, z: number)
    local b = function(i: number): number
        return bit32.band(bit32.rshift(params, i * 8), 0xFF)
    end
    local function effect(px: number, py: number, pz: number)
        local cx, cy = field.center(px, py)
        battle.effect(Vec3.new(cx, cy, pz), b(0), b(1), b(2), b(3))
    end
    n = bit32.band(n, 0xFF)
    if bit32.band(n, 0x80) ~= 0 then
        local c = data.rules.field_regions[bit32.band(n, 0x7F)]
        if not c then
            error(string.format("whole-field region %#x reads past byte_8019C34", n))
        end
        for py = 3, 1, -1 do
            for px = 6, 1, -1 do
                if field.check(px, py, c.require, c.forbid) then
                    effect(px, py, 0)
                end
            end
        end
        return
    end
    if n == 0 then
        return
    end
    local direction = 1 - 2 * side
    for _, o in region(n) do
        local px, py = x + o[1] * direction, y + o[2]
        if field.valid(px, py) then
            effect(px, py, z)
        end
    end
end

-- `sub_80DB48A`: crack (`object_crackPanel`) each panel of region `n`
-- around (x, y), its offsets as they are (not turned to a side).
function panels.crack_region(x: number, y: number, n: number)
    for _, o in region(n) do
        field.crack(x + o[1], y + o[2])
    end
end

return panels
