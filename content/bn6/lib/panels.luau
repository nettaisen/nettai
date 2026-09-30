--!strict
-- Panel lists as the original's routines build them. A panel is packed as
-- the game packs it, x | y << 4 (so a list fits a `u8[18]` state field);
-- `panels.x` and `panels.y` unpack it.

local order = require("./order")

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

-- `sub_8000C72`: shuffle the first `n` entries of `list` in place over `count` rounds of two `rng_positive` draws
-- (`battle.shuffle_order`). (With no entries the game divides by zero; with no rounds it loops about 2^32 times.)
function panels.shuffle(list: { number }, n: number, count: number)
    if n == 0 then
        error("sub_8000C72 shuffles an empty list (it divides by zero)")
    end
    if count == 0 then
        error("sub_8000C72 with no rounds loops 2^32 times")
    end
    order.apply(list, 0, battle.shuffle_order(n, count))
end

return panels
