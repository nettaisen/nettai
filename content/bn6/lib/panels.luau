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

return panels
