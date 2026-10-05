--!strict
-- EXE5's Poltrgst (the dimming handler's subtype 49): EXE5's own. Its controller (effect object #0x72, 0x080E8C1C):
-- the usual dimming phases, whose effect (0x080E8C60), when the field has an obstacle Poltergeist takes, sends out a
-- stand-in of the user (actor object #0x58, 0x080C38E8: drawn as the user in animation 12 while the user vanishes),
-- which raises the poltergeist (effect object #0x73, 0x080E8CFC): it takes every such obstacle (the field-object
-- registry's eight slots in order, but those whose identities aren't `throwable`: EXE5's NameIDs 0xDA, 0xD3, 0xD2,
-- 0xE5, 0xE4 and 0xE7), front to back as its side faces (on one column, top row first), and throws each at a panel
-- (`obstacle.throw`, `sub_800F6AC`: the obstacle rises, shakes 30 ticks, 20 more for each before it, and flies onto
-- the panel, breaking there with a hit of the chip's damage); the panel a random one with the other side's navi on
-- it, else one of the other side's but its own, else any (`sub_800F6C6`, a GetPositiveSignedRNG2 draw each), or, by
-- its first parameter (the chip's: 0 for Poltrgst), the nearest such in front (0x080E8E7C). Once the field has none
-- left, 30 ticks later it ends, and the stand-in gives the user back.

local dimming_chips = require("@exelib/dimming")

local poltergeist = {}

-- The obstacles Poltergeist takes (0x080E8CA0, 0x080E8D1C): registered field objects but those not `throwable`.
local function takes(o: Object): boolean
    return obstacle.throwable(o)
end

local function any_taken(): boolean
    for _, o in field.all_objects() do
        if takes(o) then
            return true
        end
    end
    return false
end

-- ---- The poltergeist -----------------------------------------------------------------------------------------------

export type PoltergeistState = {
    -- Its stand-in, waiting on it (its +0x40 pointer to the stand-in's flag).
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

-- `sub_800F6C6` (EXE5's 0x0800D5FE): a random panel with the other side's navi; else one of the other side's but its
-- own; else any.
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

-- 0x080E8E7C: of the panels with the other side's navi but its own, those in front of it as it faces, the nearest
-- by columns, then (as the original compares) the one whose row is under the best's row difference, then the
-- upper; (0, 0) when there is none.
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

local function poltergeist_gone(me: Object)
    local holder = (me.state :: PoltergeistState).holder
    if holder and holder.active then
        (holder.state :: { busy: boolean }).busy = false
    end
    me:free()
end

-- 0x080E8D1C: take the obstacles and sort them, front first as it faces (the larger column facing right), on a
-- column the upper; none: gone.
local function take(me: Object)
    local s = me.state :: PoltergeistState
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
    local s = me.state :: PoltergeistState
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
    if not any_taken() then
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

local POLTERGEIST = define.kind {
    id = "poltrgst/poltergeist",
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
                error(string.format("Poltrgst's poltergeist: phase %#x reads past 0x080E8DD0", p))
            end
        else
            poltergeist_gone(me)
        end
    end,
}

-- ---- The stand-in --------------------------------------------------------------------------------------------------

export type StandInState = {
    -- The poltergeist runs (+0x0C, which it clears).
    busy: boolean,
    -- Its controller, waiting on it (its +0x60 pointer to the controller's flag).
    controller: Object?,
    nearest: boolean,
}

local STAND_IN_ANIM = 12

-- 0x080C39CA: the poltergeist out; when it is done, the parts off, the user back, gone, the controller on.
local function stand(me: Object)
    local s = me.state :: StandInState
    if me.phase_init == 0 then
        -- (Its position is the spawner's registers: its row, its side and 0.)
        local o = battle.spawn(POLTERGEIST, Vec3.new(me.panel_y, me.alliance, 0))
        if o then
            o.panel_x, o.panel_y = me.panel_x, me.panel_y
            -- (Its element byte is the stand-in's side, as the spawner's registers leave it.)
            o.element = me.alliance
            o.related1 = me
            o.alliance, o.flip = me.alliance, me.flip
            o:set_damage_word(int.u32(me.damage + me.stamina * 0x10000))
            local ps = o.state :: PoltergeistState
            ps.holder, ps.nearest = me, s.nearest
            s.busy = true
        end
        me.phase_init = 4
    end
    if s.busy then
        return
    end
    local user = me.related1 :: Object
    me:remove_parts_of(user)
    me:set_lifecycle("destroy")
    dimming.show_user(user)
    local c = s.controller
    if c and c.active then
        (c.state :: { busy: boolean }).busy = false
    end
end

-- 0x080C390C: drawn as the user in animation 12 where the user is, with its palette and (a player's) parts; the user
-- vanishes, its status visuals kept.
local function stand_in_init(me: Object)
    local user = me.related1
    if not user then
        error("Poltrgst's stand-in has no user (0x080C390C reads through a null pointer)")
    end
    local sprite = me.sprite
    sprite:load_like(user)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = STAND_IN_ANIM, 0xFF
    me.visible = true
    me.pos = user.pos
    me:set_panel_from_coordinates()
    -- sub_801002C: the user's navi palette.
    sprite.palette = user.sprite.palette
    if user.actor_type == "player" then
        me:add_parts_of(user, false, true)
    end
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    dimming.hide_user_sparing(user)
    me:set_lifecycle("update")
    stand(me)
end

local STAND_IN = define.kind {
    id = "poltrgst/stand-in",
    pool = "actor",
    state = { busy = "bool", controller = "object", nearest = "bool" },
    -- 0x080C38E8.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            stand_in_init(me)
        elseif l == "update" then
            stand(me)
        else
            -- 0x080C39BE: the controller's flag cleared, freed.
            local c = (me.state :: StandInState).controller
            if c and c.active then
                (c.state :: { busy: boolean }).busy = false
            end
            me:free()
            return
        end
        me:update_sprite()
    end,
}

-- ---- The controller ------------------------------------------------------------------------------------------------

export type ControllerState = {
    -- The Atk+ bonus (+0x32): it adds to the stand-in's damage word.
    bonus: number,
    -- The stand-in is out (+0x0C, which it clears).
    busy: boolean,
    nearest: boolean,
}

-- 0x080E8C60: with an obstacle to take, the stand-in out (0x080C3A10: on the controller's panel, the element byte
-- its side, the damage word with the bonus, the user's side and facing, running while dimmed); when it is done (or
-- with none), done.
local function effect(me: Object)
    local s = me.state :: ControllerState
    if me.phase_init == 0 then
        if not any_taken() then
            dimming_chips.done(me)
            return
        end
        local user = me.related1 :: Object
        local o = battle.spawn(STAND_IN)
        if o then
            o.panel_x, o.panel_y = me.panel_x, me.panel_y
            o.element = me.alliance
            o:set_damage_word(dimming_chips.damage_plus(me, s.bonus))
            o.related1 = user
            local ss = o.state :: StandInState
            ss.controller, ss.nearest = me, s.nearest
            s.busy = true
            o.alliance, o.flip = user.alliance, user.flip
            o.run_while_dimmed = true
        end
        me.phase_init = 4
    end
    if not s.busy then
        dimming_chips.done(me)
    end
end

poltergeist.controller = define.kind {
    id = "poltrgst/controller",
    pool = "effect",
    state = { bonus = "u16", busy = "bool", nearest = "bool" },
    -- 0x080E8C1C.
    update = dimming_chips.phases { name = "Poltrgst", effect = effect },
}

-- 0x080E8CDA: the chip's `dimming` hook; `nearest`: its first parameter.
function poltergeist.hook(nearest: boolean): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(poltergeist.controller, user, spec)
        if o then
            local s = o.state :: ControllerState
            s.bonus, s.nearest = spec.bonus, nearest
        end
        return o
    end
end

return poltergeist
