--!strict
-- A gust (attack object #0x49, `sub_80CD2EC`): wind blowing along a row at
-- 10 pixels a tick, pushing what it meets (hit modifier 8, or 4 with
-- Param1 bit 0; element 5, no damage of its own). It ends when it leaves
-- the field, meets something that stops it (hit flag 0x800000), or, unless
-- Param1 bit 7 is set, reaches its own side's panels. Param1 bit 7 also
-- keeps it invisible (WindRack's), and its low bits turn it around.
--
-- A spawner can hand it a signal: an object whose content state declares
-- a u8 field `gusts`. The gust raises it (1) when it spawns and lowers it
-- (0) when it ends, and ends early once someone else lowers it (the
-- game's flag byte behind ExtraVars+4).

export type State = {
    -- ExtraVars+0: end on any hit. Its spawner (`sub_80CD488`) never sets
    -- it.
    ends_on_hit: boolean,
    -- ExtraVars+4: the object whose `gusts` flag it raises and lowers.
    signal: Object?,
}

local gust = {
    state = { ends_on_hit = "bool", signal = "object" },
}

local SPRITE = "0c-2e"
-- Its speed along the row, 16.16.
local SPEED = 0xA_0000
-- The hit flag of something that stops it.
local STOPPED = 0x80_0000
-- Param1 bits: blows anywhere and stays invisible; pushes less.
local ANYWHERE, SOFT = 0x80, 0x01
-- The panels it may blow over, by side (`byte_80CD478`): the other side's
-- (the alliance-1 owner bit 0x20 set for side 0, clear for side 1).
local ALLIANCE_1 = 0x20
local OTHER_SIDE = {
    [0] = { require = ALLIANCE_1, forbid = 0 },
    [1] = { require = 0, forbid = ALLIANCE_1 },
}

type Signalled = { gusts: number }

local function raise(signal: Object)
    (signal.state :: Signalled).gusts = 1
end

local function lower(signal: Object?)
    if signal then
        (signal.state :: Signalled).gusts = 0
    end
end

-- `sub_80CD488`: a gust on (x, y) for `alliance` and `flip`, with Param1..4
-- `params`, the damage word `damage` and an optional signal. (Its position
-- is its spawner's registers, all overwritten by its init.)
function gust.spawn(
    x: number,
    y: number,
    alliance: number,
    flip: number,
    params: { number },
    damage: number,
    signal: Object?
): Object?
    local o = battle.spawn_kind("gust", Vec3.zero, params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.alliance, o.flip = alliance, flip
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        local s = o.state :: State
        s.signal = signal
        if signal then
            raise(signal)
        end
    end
    return o
end

-- `loc_80CD45E`: stop blowing (the region goes; still drawn until freed).
local function stop(me: Object)
    me.collision.region = 0
    lower((me.state :: State).signal)
    me:set_lifecycle("destroy")
end

-- `sub_80CD414`: blow on along the row.
local function blow(me: Object)
    if bit32.btest(me.collision.hit_flags, STOPPED) then
        return stop(me)
    end
    local p = me.pos
    me.pos = Vec3.new(p.x + me.vel.x, p.y, p.z)
    -- `sub_800E24C`: the panel under it, for itself and its collision.
    me:set_panel_from_coordinates()
    local x, y = me.panel_x, me.panel_y
    me.collision.panel_x, me.collision.panel_y = x, y
    if not field.valid(x, y) then
        return stop(me)
    end
    if bit32.btest(me:param(1), ANYWHERE) then
        return
    end
    local side = OTHER_SIDE[me.alliance]
    if not side then
        error(string.format("gust alliance %d reads past byte_80CD478", me.alliance))
    end
    if not field.check(x, y, side.require, side.forbid) then
        stop(me)
    end
end

-- `sub_80CD3B0`: resolve its hits; end on a hit (if told to), when its
-- signal is lowered or the battle is over; else blow.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    local s = me.state :: State
    local ended = false
    if s.ends_on_hit and c.hit_flags ~= 0 then
        ended = true
    elseif s.signal and ((s.signal :: Object).state :: Signalled).gusts == 0 then
        ended = true
    elseif battle.over() then
        lower(s.signal)
        ended = true
    end
    if ended then
        c.region = 0
        me.visible = false
        me:set_lifecycle("destroy")
        return
    end
    local a = me.action
    if a ~= 0 then
        error(string.format("gust action %#x reads past its table", a))
    end
    blow(me)
    c:present()
end

-- `sub_80CD310`.
local function init(me: Object)
    local p1 = me:param(1)
    if not bit32.btest(p1, ANYWHERE) then
        me.visible = true
    end
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite:step()
    local turned = bit32.band(p1, 0x7F)
    sprite:set_flip(bit32.bxor(bit32.band(bit32.bxor(me.alliance, me.flip), 0x7F), p1))
    me.element = 5
    sprite.palette = p1 * 2
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    -- `object_getFlipDirection` of its side turned by Param1's low bits.
    local dir = 1 - 2 * bit32.bxor(bit32.bxor(me.alliance, turned), me.flip)
    me.vel = Vec3.new(int.mul32(dir, SPEED), me.vel.y, me.vel.z)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(0x1E, 5, if bit32.btest(p1, SOFT) then 4 else 8)
    -- (The game then stores 0 through the set-up's return value, which is
    -- its damage, not a pointer: a write to low memory that does nothing.)
    me.collision:present()
    me.lifecycle = "update"
    tick(me)
end

-- `sub_80CD2EC`.
function gust.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
        if not me.active then
            return
        end
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
        return
    end
    me:update_sprite()
end

return gust
