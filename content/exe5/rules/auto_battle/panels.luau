--!strict
-- The panel searches EXE5's auto battle AI runs (./ai, ./place): where to step, where a chip reaches from. Most are
-- EXE6's routines (the same code in both games); the EXE5-only ones are named by their EXE5 address. A search that finds
-- nothing returns nil; one that finds several picks one with a battle RNG draw (`GetPositiveSignedRNG2`, modulo
-- how many), in the order the original lists them.

local data = require("./data")

local panels = {}

-- A panel's place packed as the original packs it in a list: x | y << 4.
local function pack(x: number, y: number): number
    return bit32.bor(bit32.band(x, 0xF), bit32.lshift(bit32.band(y, 0xF), 4))
end

local function unpack(p: number): (number, number)
    return bit32.band(p, 7), bit32.rshift(p, 4)
end

-- One of `list` (packed places) at random, unpacked; nil for none (no draw).
local function pick(list: { number }): (number?, number?)
    if #list == 0 then
        return nil, nil
    end
    return unpack(list[battle.rng_positive() % #list + 1])
end
panels.pick = pick

-- `object_getAllianceDirection`: +1 for side 0, -1 for side 1.
function panels.side_direction(side: number): number
    return 1 - 2 * bit32.band(side, 1)
end

-- `object_getFlipDirection`: +1 facing right, -1 facing left.
function panels.flip_direction(side: number, flip: number): number
    return 1 - 2 * bit32.band(bit32.bxor(side, flip), 1)
end

-- `object_getEnemyDirection`: toward the navi's enemies (its side's direction).
function panels.enemy_direction(navi: Object): number
    return panels.side_direction(navi.alliance)
end

-- `object_getFrontDirection`: the way the navi faces.
function panels.front_direction(navi: Object): number
    return panels.flip_direction(navi.alliance, navi.flip)
end

-- The panels a side's navi may step onto (`sub_8010368`'s table, by side and whether it wears AirShoes; the
-- byte_8015C84 rows `sub_8015C48` uses are its first two).
local STEP: { { number } } = {
    { 0x10, 0x0F8800A0 },
    { 0x30, 0x0F880080 },
    { 0x00, 0x0F8800A0 },
    { 0x20, 0x0F880080 },
}

-- `sub_8010368`: the conditions of a panel `navi` may step onto: (require, forbid).
function panels.step_condition(navi: Object): (number, number)
    local row = STEP[(if navi:status("air_shoe") then 2 else 0) + bit32.band(navi.alliance, 1) + 1]
    return row[1], row[2]
end

-- `object_getPanelsFiltered` (rows 3 to 1, columns 6 to 1), and without the navi's own panel
-- (`object_getPanelsExceptCurrentFiltered`).
local function filtered(require: number, forbid: number, except: Object?): { number }
    local out = {}
    for y = 3, 1, -1 do
        for x = 6, 1, -1 do
            if field.check(x, y, require, forbid) and not (except and x == except.panel_x and y == except.panel_y) then
                table.insert(out, pack(x, y))
            end
        end
    end
    return out
end
panels.filtered = filtered

-- `object_getPanelsInRowFiltered`: row `y`'s panels meeting the conditions, left to right.
local function in_row(y: number, require: number, forbid: number): { number }
    local out = {}
    for x = 1, 6 do
        if field.check(x, y, require, forbid) then
            table.insert(out, pack(x, y))
        end
    end
    return out
end

-- `sub_8015C48`: any panel of the navi's own area but its own (`byte_8015C84`), at random; none: its own panel.
function panels.random_own(navi: Object): (number, number)
    local row = STEP[bit32.band(navi.alliance, 1) + 1]
    local x, y = pick(filtered(row[1], row[2], navi))
    if x == nil then
        return navi.panel_x, navi.panel_y
    end
    return x :: number, y :: number
end

-- The other side's bodies a side looks for (`byte_8015F38`: panel flag 0x04000000 for side 0, 0x08000000 for 1).
local ENEMY_BODIES: { number } = { 0x04000000, 0x08000000 }

-- `sub_8015EC6`: a panel of the navi's own area in an enemy's row, at random; nil if no enemy stands anywhere, or
-- none of the row's panels is the navi's to step on. The routine means the row of the enemy nearest the navi's own
-- row, but its loop reads the first of the panels an enemy stands on (rows 3 to 1, columns 6 to 1, the navi's own
-- left out) every time: that panel's row.
function panels.row_of_nearest_enemy(navi: Object): (number?, number?)
    local enemies = filtered(ENEMY_BODIES[bit32.band(navi.alliance, 1) + 1], 0, navi)
    if #enemies == 0 then
        return nil, nil
    end
    local _, row = unpack(enemies[1])
    local cond = STEP[bit32.band(navi.alliance, 1) + 1]
    return pick(in_row(row, cond[1], cond[2]))
end

-- The panels behind a body a side's searches stop at (`byte_8015D78`: flags 0x01800000 for side 0, 0x02800000 for 1).
local BLOCKING: { number } = { 0x01800000, 0x02800000 }

-- `sub_8015CC0`: from (x, y) toward the navi's own side, past `skip` blocking panels (a body of its own side or a
-- neutral one), the panels meeting the conditions before the next one: one at random, and the last blocking panel
-- the walk met (nil x when no panel turned up).
function panels.behind(navi: Object, x: number, y: number, require: number, forbid: number, skip: number): (number?, number?, number, number)
    local back = -panels.side_direction(navi.alliance)
    local blocking = BLOCKING[bit32.band(navi.alliance, 1) + 1]
    local found = {}
    local hits, lx, ly = 0, 0, 0
    x += back
    while true do
        if bit32.band(field.flags(x, y), blocking) ~= 0 then
            lx, ly = x, y
            hits += 1
            if hits > skip then
                break
            end
        end
        if hits == skip and field.check(x, y, require, forbid) then
            table.insert(found, pack(x, y))
        end
        x += back
        if not field.valid(x, y) then
            break
        end
    end
    local px, py = pick(found)
    return px, py, lx, ly
end

-- `_GetRandomRelativePanelFiltered`: of the panels `list` names from (x, y) (dx toward side `side`'s enemies), those
-- meeting the conditions: one at random, and how many there were.
function panels.random_relative(x: number, y: number, require: number, forbid: number, list: { { number } }, side: number): (number?, number?, number)
    local dir = panels.side_direction(side)
    local found = {}
    for _, d in list do
        local px, py = x + d[1] * dir, y + d[2]
        if field.check(px, py, require, forbid) then
            table.insert(found, pack(px, py))
        end
    end
    local px, py = pick(found)
    return px, py, #found
end

-- `GetRandomRelativePanelFiltered` (EXE5's 0x08012C26): the same over the panel offset list `index` (data.offsets).
function panels.random_offset(x: number, y: number, require: number, forbid: number, index: number, side: number): (number?, number?, number)
    return panels.random_relative(x, y, require, forbid, data.offsets[index], side)
end

-- EXE5's 0x08012C38: the panels from which a chip of reach list `range` (data.ranges) reaches (x, y): the list
-- turned the other side's way.
function panels.reaching(x: number, y: number, require: number, forbid: number, range: number, side: number): (number?, number?, number)
    return panels.random_relative(x, y, require, forbid, data.ranges[range], bit32.bxor(side, 1))
end

-- `sub_8015D80`: of the panels `list` names from (x, y) (dx toward the navi's enemies), those meeting the
-- conditions: one at random.
function panels.random_toward(navi: Object, x: number, y: number, require: number, forbid: number, list: { { number } }): (number?, number?)
    local dir = panels.enemy_direction(navi)
    local found = {}
    for _, d in list do
        local px, py = x + d[1] * dir, y + d[2]
        if field.check(px, py, require, forbid) then
            table.insert(found, pack(px, py))
        end
    end
    return pick(found)
end

-- The panels above and below (0x08012D7C).
local UP_DOWN: { { number } } = { { 0, -1 }, { 0, 1 } }

-- EXE5's 0x08012D48: the panel above or below the navi it may step onto (`byte_8015C84`'s rows), at random.
function panels.random_up_down(navi: Object): (number?, number?)
    local row = STEP[bit32.band(navi.alliance, 1) + 1]
    return panels.random_toward(navi, navi.panel_x, navi.panel_y, row[1], row[2], UP_DOWN)
end

-- `sub_800D1F0`: in each row, the front panel of `side`'s area (as `side` faces with `flip`: from the far end back
-- to the first panel `side` holds), if it meets the conditions; top to bottom.
function panels.area_fronts(side: number, flip: number, require: number, forbid: number): { number }
    local back = -panels.flip_direction(side, flip)
    local start = 6 - 5 * bit32.band(bit32.bxor(side, flip), 1)
    local out = {}
    for y = 1, 3 do
        local x = start
        while true do
            local p = field.panel(x, y)
            if p == nil then
                break
            end
            if p.alliance == side then
                if field.check(x, y, require, forbid) then
                    table.insert(out, pack(x, y))
                end
                break
            end
            x += back
        end
    end
    return out
end

-- The panels a side's front panel faces that a navi in auto battle won't step behind (0x080131A0: a body of the side's own,
-- or a neutral one).
local OWN_BODIES: { number } = { 0x01800000, 0x02800000 }

-- EXE5's 0x08013126: of the front panels of the navi's own area it may step onto (`sub_800D1F0`), those whose next
-- panel forward is free of its side's and neutral bodies and is one a body may stand on (flag 0x10000): one at
-- random.
function panels.front_line(navi: Object, side: number, flip: number): (number?, number?)
    local require, forbid = panels.step_condition(navi)
    local fronts = panels.area_fronts(side, flip, require, forbid)
    local found = {}
    for _, p in fronts do
        local x, y = unpack(p)
        local flags = field.flags(x + panels.flip_direction(side, flip), y)
        if bit32.band(flags, OWN_BODIES[bit32.band(side, 1) + 1]) == 0 and bit32.band(flags, 0x10000) ~= 0 then
            table.insert(found, p)
        end
    end
    return pick(found)
end

return panels
