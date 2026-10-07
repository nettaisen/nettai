--!strict
-- The poltergeist, as EXE5 and EXE4 share it (effect object #0x73: EXE5's 0x080E8CFC, which its Poltrgst's stand-in
-- raises; EXE4's 0x080EA13C, which JunkMan raises): it takes every obstacle on the field it can throw (the field-object
-- registry's eight slots in order, but those whose identities aren't `throwable`: EXE5's NameIDs 0xDA, 0xD3, 0xD2,
-- 0xE5, 0xE4 and 0xE7; EXE4's code tests kinds instead, 0x080EA16C and 0x0800B3E8: attack objects #0x4C and #0x8C and
-- effect object #0x6E, none ported yet, whose identities are to state it), front to back as its side faces (on one
-- column, top row first), and throws each at a panel (`obstacle.throw`, `sub_800F6AC`: the obstacle rises, shakes 30
-- ticks, 20 more for each before it, and flies onto the panel, breaking there with a hit of its damage); the panel a
-- random one with the other side's navi on it, else one of the other side's but its own, else any (`sub_800F6C6`, a
-- GetPositiveSignedRNG2 draw each), or, by its first parameter (EXE5's Poltrgst 0, random; EXE4's JunkMan 1, nearest),
-- the nearest such in front (EXE5's 0x080E8E7C, EXE4's 0x080DEF1A). Once the field has none left, 30 ticks later it
-- ends and tells who raised it (its holder's state `busy`, its +0x40 pointer to the flag). With none to take it ends at
-- once.
--
-- Each game makes its kind (`poltergeist.make`).

local poltergeist = {}

-- The obstacles it takes: registered field objects but those not `throwable`.
local function takes(o: Object): boolean
    return obstacle.throwable(o)
end

-- The field has an obstacle it takes (EXE5's 0x080E8CA0, EXE4's 0x0800B3E8).
function poltergeist.any_taken(): boolean
    for _, o in field.all_objects() do
        if takes(o) then
            return true
        end
    end
    return false
end

export type State = {
    -- Who raised it, waiting on it (its +0x40 pointer to the holder's flag: the holder's state's `busy`).
    holder: Object?,
    -- The obstacles it takes, sorted (+0x60), and how many (+0x0C).
    taken: { Object },
    count: number,
    -- Its first parameter: the nearest target in front rather than a random one.
    nearest: boolean,
}

-- Its throws: the first shakes 30 ticks, each next 20 more; it lingers 30 ticks once the field is clear.
local FIRST_SHAKE, SHAKE_STEP, LINGER = 30, 20, 30
-- Panels with the other side's navi on them, by the thrower's side (0x0800D680), else the other side's panels
-- (0x0800D690: a side-1 panel's bit, required by side 0 and forbidden by side 1).
local NAVIS: { [number]: number } = { [0] = 0x0400_0000, [1] = 0x0800_0000 }
local SIDE_1 = 0x20
local ENEMY_PANELS: { [number]: { number } } = { [0] = { SIDE_1, 0 }, [1] = { 0, SIDE_1 } }

-- The field's panels whose flags have every `require` bit and no `forbid` bit, rows 3 to 1 and columns 6 to 1
-- (`object_getPanelsFiltered`), but `skip`'s own (`object_getPanelsExceptCurrentFiltered`).
local function panels(require_bits: number, forbid: number, skip: Object?): { { number } }
    local out = {}
    for y = 3, 1, -1 do
        for x = 6, 1, -1 do
            local flags = field.flags(x, y)
            if not bit32.btest(flags, forbid) and bit32.band(flags, require_bits) == require_bits then
                if not (skip and skip.panel_x == x and skip.panel_y == y) then
                    table.insert(out, { x, y })
                end
            end
        end
    end
    return out
end

-- One of `list` at random (GetPositiveSignedRNG2, the remainder of its division by the count: `svc 6`).
local function pick(list: { { number } }): (number, number)
    if #list == 0 then
        error("sub_800F6C6 divides by zero: the field has no panel")
    end
    local p = list[battle.rng_positive() % #list + 1]
    return p[1], p[2]
end

-- `sub_800F6C6` (EXE5's 0x0800D5FE, EXE4's 0x0800B2DC): a random panel with the other side's navi; else one of the
-- other side's but its own; else any.
local function random_target(me: Object): (number, number)
    local side = me.alliance
    local navis = panels(NAVIS[side], 0, nil)
    if #navis > 0 then
        return pick(navis)
    end
    local enemy = ENEMY_PANELS[side]
    local others = panels(enemy[1], enemy[2], me)
    if #others > 0 then
        return pick(others)
    end
    return pick(panels(0, 0, nil))
end

-- EXE5's 0x080E8E7C (EXE4's 0x080DEF1A): of the panels with the other side's navi but its own, those in front of it as
-- it faces, the nearest by columns, then (as the original compares) the one whose row is under the best's row
-- difference, then the upper; (0, 0) when there is none.
local function nearest_target(me: Object): (number, number)
    local found = panels(NAVIS[me.alliance], 0, me)
    local facing_left = bit32.bxor(me.alliance, me.flip) ~= 0
    local best_x, best_y, best_dx, best_dy = 0, 0, 0xFF, 0xFF
    for _, p in found do
        local x, y = p[1], p[2]
        local ahead = if facing_left then me.panel_x > x else x > me.panel_x
        if ahead then
            local dx, dy = math.abs(x - me.panel_x), math.abs(y - me.panel_y)
            local better: boolean
            if dx ~= best_dx then
                better = dx < best_dx
            elseif y ~= best_dy then
                -- (0x080E8F1A compares the row itself with the best's row difference.)
                better = y < best_dy
            else
                better = y < best_y
            end
            if better then
                best_x, best_y, best_dx, best_dy = x, y, dx, dy
            end
        end
    end
    return best_x, best_y
end

export type Poltergeist = {
    kind: Kind,
    spawn: (holder: Object, damage: number, nearest: boolean) -> Object?,
}

function poltergeist.make(id: string): Poltergeist
    -- Who raised it is told it is done; it is freed (EXE5's 0x080E8E46).
    local function gone(me: Object)
        local holder = (me.state :: State).holder
        if holder and holder.active then
            (holder.state :: { busy: boolean }).busy = false
        end
        me:free()
    end

    -- 0x080E8D1C: take the obstacles and sort them, front first as it faces (the larger column facing right), on a
    -- column the upper; none: gone.
    local function take(me: Object)
        local s = me.state :: State
        local n = 0
        for _, o in field.all_objects() do
            if takes(o) then
                n += 1
                s.taken[n] = o
            end
        end
        s.count = n
        if n == 0 then
            me:set_lifecycle("destroy")
            return
        end
        local facing_left = bit32.bxor(me.alliance, me.flip) ~= 0
        for i = 1, n - 1 do
            for j = i + 1, n do
                local a, b = s.taken[i], s.taken[j]
                local ax, bx = a.panel_x, b.panel_x
                local swap: boolean
                if (if facing_left then bx > ax else ax > bx) then
                    swap = false
                elseif ax ~= bx then
                    swap = true
                else
                    swap = not (a.panel_y < b.panel_y)
                end
                if swap then
                    s.taken[i], s.taken[j] = b, a
                end
            end
        end
        me:set_lifecycle("update")
    end

    -- 0x080E8DD8: throw each, then wait until the field has none left.
    local function throw_all(me: Object)
        local s = me.state :: State
        if me.phase_init == 0 then
            local damage = int.u32(me.damage + me.stamina * 0x10000)
            local i = me.timer2
            repeat
                local x, y: number
                if s.nearest then
                    x, y = nearest_target(me)
                else
                    x, y = random_target(me)
                end
                obstacle.throw(s.taken[i + 1], me.alliance, x, y, FIRST_SHAKE + SHAKE_STEP * i, damage)
                i += 1
                me.timer2 = i
            until i >= s.count
            me.phase_init = 4
        end
        if not poltergeist.any_taken() then
            me.phase, me.phase_init = 4, 0
        end
    end

    -- 0x080E8E28: 30 ticks, then gone.
    local function linger(me: Object)
        if me.phase_init == 0 then
            me.timer = LINGER
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = int.u16(t)
        if t < 0 then
            me:set_lifecycle("destroy")
        end
    end

    local kind = new.kind {
        id = id,
        pool = "effect",
        state = { holder = "object", taken = "object[8]", count = "u8", nearest = "bool" },
        -- 0x080E8CFC.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                take(me)
            elseif l == "update" then
                local p = me.phase
                if p == 0 then
                    throw_all(me)
                elseif p == 4 then
                    linger(me)
                else
                    error(string.format("the poltergeist: phase %#x reads past 0x080E8DD0", p))
                end
            else
                gone(me)
            end
        end,
    }

    -- EXE5's 0x080C39CA (its stand-in's), EXE4's 0x080EA294: a poltergeist raised by `holder`, on its panel, of its side
    -- and facing, the element byte its side (as the spawner's registers leave it), with the damage word `damage`;
    -- `nearest`: its first parameter. The holder's state's `busy` is set (the flag it was handed). (Its position is the
    -- spawner's registers: its row, its side and 0.)
    local function spawn(holder: Object, damage: number, nearest: boolean): Object?
        local o = battle.spawn(kind, Vec3.new(holder.panel_y, holder.alliance, 0))
        if not o then
            return nil
        end
        o.panel_x, o.panel_y = holder.panel_x, holder.panel_y
        o.element = holder.alliance
        o.related1 = holder
        o.alliance, o.flip = holder.alliance, holder.flip
        o:set_damage_word(damage)
        local s = o.state :: State
        s.holder, s.nearest = holder, nearest
        ;(holder.state :: { busy: boolean }).busy = true
        return o
    end

    return { kind = kind, spawn = spawn }
end

return poltergeist
