--!strict
-- Where the Gregar and Falzar chips' attacks land (docs/engine/beast-chips.md):
-- a panel of the other side's area, by preference one with an enemy on it,
-- and not the last one aimed at. Gregar's summon drops its rocks there
-- (chips/gregar/summon), Falzar's controller its feathers
-- (chips/falzar/controller). The original has the routine twice, the same
-- code in each: the Japanese ROMs' 0x080C40E6 (Gregar's summon) and
-- 0x080EE0A8 (Falzar's controller) in the Falzar ROM, 0x080C5946 and
-- 0x080EF3D8 in the Gregar ROM. (The US ROMs have neither.)

local panels = require("@exelib/panels")

local targets = {}

-- The panels with an enemy's body on them, of the other side's area, by
-- `me`'s side: (require, forbid) (0x080C4198).
local ENEMY_PANELS: { [number]: { number } } = { [0] = { 0x04010020, 0 }, [1] = { 0x08010000, 0x20 } }
-- Every panel of the other side's area (0x080C41AC).
local AREA_PANELS: { [number]: { number } } = { [0] = { 0x00010020, 0 }, [1] = { 0x00010000, 0x20 } }
-- Of sixteen draws, those that aim at an enemy's panel.
local ENEMY_ODDS = 9

-- (x, y) of a packed panel, unless it is the last aim.
local function fresh(p: number, last_x: number, last_y: number): (number?, number?)
    local x, y = panels.x(p), panels.y(p)
    if x ~= last_x or y ~= last_y then
        return x, y
    end
    return nil, nil
end

-- The panels meeting (require, forbid), but `me`'s own, written over the
-- start of `buf` (the routine's stack buffer, which keeps what an earlier
-- list left past the new one's end): how many.
local function list_into(me: Object, buf: { number }, filter: { number }): number
    local list = panels.filtered_except(me, filter[1], filter[2])
    for i, p in list do
        buf[i] = p
    end
    return #list
end

-- The panel to aim at, given the last one (`last_x`, `last_y`; 0 before
-- the first): with nine draws in sixteen, the first of the enemies' panels
-- once shuffled, or the second if the first is the last aim; else, or with
-- no enemy to aim at or only the last aim, the same from the whole of the
-- other side's area. nil when that gives nothing. One RNG draw for the
-- odds when an enemy stands there, and two for each swap of a shuffle
-- (as many swaps as panels: `sub_8000C72`).
function targets.pick(me: Object, last_x: number, last_y: number): (number?, number?)
    local side = bit32.band(me.alliance, 1)
    local buf: { number } = {}
    local n = list_into(me, buf, ENEMY_PANELS[side])
    if n ~= 0 and bit32.band(battle.rng_positive(), 0xF) < ENEMY_ODDS then
        panels.shuffle(buf, n, n)
        local x, y = fresh(buf[1], last_x, last_y)
        if x then
            return x, y
        end
        if n > 1 then
            x, y = fresh(buf[2], last_x, last_y)
            if x then
                return x, y
            end
        end
    end
    n = list_into(me, buf, AREA_PANELS[side])
    if n == 0 then
        return nil, nil
    end
    panels.shuffle(buf, n, n)
    local x, y = fresh(buf[1], last_x, last_y)
    if x then
        return x, y
    end
    -- The second entry, whether or not the list has one. A side's area
    -- always keeps a whole column, so the list has two panels at least in
    -- a battle; with one, the byte read is whatever the first list (or an
    -- earlier routine's stack) left there.
    local second = buf[2]
    if second == nil then
        error("the beast chips' aim (0x080C40E6) reads its stack buffer's second byte, which no list wrote")
    end
    return fresh(second, last_x, last_y)
end

return targets
