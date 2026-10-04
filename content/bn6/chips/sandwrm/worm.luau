--!strict
-- SandWrm's worm (the original's attack object #0xCB, `sub_80DEA7C`) and
-- the chips' effect (instant chip effect 12, `sub_80DEDE0`): behind each of
-- the other side's navis that has an open panel there, a worm bursts out
-- of a sand hole (hole.luau) and arcs 22 pixels high over its panel (and
-- the navi) toward the user, into a second hole, throwing sand
-- (spray.luau). It hits what it passes; the holes close 20 ticks after it
-- leaves and after it lands. It stands still while dimmed. Its level (the
-- chip's: 0 to 2) is its palette.

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local trajectory = require("../../lib/trajectory")
local hole = require("./hole")
local spray = require("./spray")

local worm = {}

export type State = {
    -- ExtraVars: the panel it lands on, its level, the last animation
    -- set, ticks into the arc, the arc's length, the angle's step and the
    -- angle (high byte: the sine table's index), the arc's length again,
    -- the panel it last hit from, its holes and their timers.
    target_x: number,
    target_y: number,
    level: number,
    last_anim: number,
    ticks: number,
    length: number,
    step: number,
    angle: number,
    duration: number,
    last_x: number,
    last_y: number,
    start_hole: Object?,
    end_hole: Object?,
    start_timer: number,
    end_timer: number,
}

local SPRITE = asset.sprite("bn6:sand-worm")
local EMERGE_SOUND, DIVE_SOUND = asset.sound("bn6:sand-worm"), asset.sound("bn6:sand-worm-2")
-- Where it can come out (`byte_80DEE44`, by side: required and forbidden
-- flags): an open panel, not the other side's body or object's.
local REQUIRE = 0x10000
local FORBID = { [0] = 0x05800000, [1] = 0x0A800000 }
-- Its collision: a thrown attack that hits navis, hit modifier 3.
local HIT_MOD = 3
-- By level: the arc's length (`byte_80DEA50`) and the holes' timers
-- (`byte_80DEA70`); the same for all three.
local LENGTH: { [number]: number } = { [0] = 0x10, 0x10, 0x10 }
local HOLE_TICKS: { [number]: { number } } = { [0] = { 0x14, 0x14 }, { 0x14, 0x14 }, { 0x14, 0x14 } }
-- The sand it throws (`byte_80DEDD8`, its looks; the fourth's Param3 1,
-- which nothing reads; `byte_80DEA58`: delays, the same for all three
-- levels).
local SPRAYS = {
    { look = 0, delay = 0x06 },
    { look = 0, delay = 0x0B },
    { look = 0, delay = 0x10 },
    { look = 0, delay = 0x15 },
    { look = 1, delay = 0x1A },
}
-- The arc's height, in units of the sine table (`byte_80DEE58`).
local HEIGHT = 0x16
-- A hit registered by something (`dword_80DEB6C`).
local HIT = 0xFF800000

-- The actions.
local EMERGE, ARC, DIVE = 0, 4, 8

-- A by-level table's entry.
local function by_level<T>(t: { [number]: T }, s: State, what: string): T
    local v = t[s.level]
    if v == nil then
        error(string.format("sand worm level %d reads past %s", s.level, what))
    end
    return v
end

-- `sub_80DED80`: the sand, from where it is.
local function throw_sand(me: Object, s: State)
    for _, sand in SPRAYS do
        spray.spawn(me, s.level, sand.look, sand.delay)
    end
end

-- `sub_80DECBC`: turned toward the user; the arc to the panel behind.
local function aim(me: Object, s: State)
    local holes = by_level(HOLE_TICKS, s, "byte_80DEA70")
    s.start_timer, s.end_timer = holes[1], holes[2]
    me.flip = 1
    s.target_x = me.panel_x + me.facing
    s.target_y = me.panel_y
    s.duration = by_level(LENGTH, s, "byte_80DEA50")
    -- sub_8114F6A
    local n = s.duration
    local tx, ty = field.center(s.target_x, s.target_y)
    local p = me.pos
    me.vel = Vec3.new(int.tdiv(int.i32(tx - p.x), n), int.tdiv(int.i32(ty - p.y), n), int.tdiv(int.i32(0 - p.z), n))
    s.step = int.tdiv(0x8000, n)
    me.pos = Vec3.new(p.x, p.y, 0)
    s.ticks = 0
    s.length = s.duration
    s.angle = 0
    me.anim = 1
    s.last_anim = 1
    throw_sand(me, s)
end

-- `sub_80DEAA8`.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = 1, 1
    sprite:set_animation(1)
    sprite.palette = s.level
    me:set_coordinates_from_panel()
    if not me:create_collision() then
        me:free()
        return false
    end
    me:setup_collision(collision.thrown, collision.hits_navis, HIT_MOD)
    local c = me.collision
    c:set_region(regions.single)
    c:set_hit_effect(nil)
    c:present()
    aim(me, s)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
    return true
end

-- The start hole's timer: it closes when it runs out.
local function start_hole_timer(s: State)
    local left = s.start_timer - 1
    s.start_timer = left
    if left <= 0 then
        s.start_timer = 0
        hole.close(s.start_hole)
        s.start_hole = nil
    end
end

-- `sub_80DEB70`: out of a hole.
local function emerge(me: Object, s: State)
    s.start_hole = hole.spawn(me.related1 :: Object, me.panel_x, me.panel_y, me)
    battle.play_sound(EMERGE_SOUND)
    me.action = ARC
    me.phase = 0
end

-- `sub_8114F52`: which quarter of the arc (0 to 3).
local function quarter(ticks: number, length: number): number
    local q = int.tdiv(ticks, int.tdiv(length, 4))
    return if q == 4 then 3 else q
end

-- `sub_80DED58`: its region comes back on each new panel.
local function rearm(me: Object, s: State)
    if me.panel_x == s.last_x and me.panel_y == s.last_y then
        return
    end
    s.last_x, s.last_y = me.panel_x, me.panel_y
    me.collision:set_region(regions.single)
end

-- `sub_80DEB94`: along the arc; into the second hole at its end.
local function arc(me: Object, s: State)
    start_hole_timer(s)
    local p = me.pos
    local sine = int.u16(trajectory.sine(bit32.rshift(s.angle, 8)))
    me.pos = Vec3.new(p.x + me.vel.x, p.y, bit32.lshift(sine * HEIGHT, 8))
    local anim = quarter(s.ticks, s.length) + 1
    if anim ~= s.last_anim then
        me.anim = anim
        s.last_anim = anim
    end
    s.angle = s.angle + s.step
    s.ticks = s.ticks + 1
    if s.ticks == s.length then
        s.end_hole = hole.spawn(me.related1 :: Object, me.panel_x, me.panel_y, me)
        me.anim = 0
        battle.play_sound(DIVE_SOUND)
        local q = me.pos
        me.pos = Vec3.new(q.x, q.y, 0)
        me.action = DIVE
        me.phase = 0
    end
    me:set_panel_from_coordinates()
    rearm(me, s)
    me:update_collision_panels()
end

-- `sub_80DEC3C`: under; the holes close.
local function dive(me: Object, s: State)
    start_hole_timer(s)
    me.collision:set_region(nil)
    me.visible = false
    local left = s.end_timer - 1
    s.end_timer = left
    if left > 0 then
        return
    end
    s.end_timer = 0
    hole.close(s.end_hole)
    s.end_hole = nil
    me:set_lifecycle("destroy")
end

-- `sub_80DEB1C`.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        me:set_lifecycle("destroy")
        return
    end
    if bit32.btest(c.hit_flags, HIT) then
        c:set_region(nil)
    end
    if battle.dimmed() then
        return
    end
    local s = me.state :: State
    local a = me.action
    if a == EMERGE then
        emerge(me, s)
    elseif a == ARC then
        arc(me, s)
    elseif a == DIVE then
        dive(me, s)
    else
        error(string.format("sand worm action %#x reads past its table", a))
    end
    c:present()
    me:highlight_collision_panels()
end

worm.kind = define.kind {
    id = "bn6:sandwrm/worm",
    pool = "attack",
    state = {
        target_x = "u8",
        target_y = "u8",
        level = "u8",
        last_anim = "u8",
        ticks = "u16",
        length = "u16",
        step = "u16",
        angle = "u16",
        duration = "u16",
        last_x = "u8",
        last_y = "u8",
        start_hole = "object",
        end_hole = "object",
        start_timer = "u16",
        end_timer = "u16",
    },
    -- `sub_80DEA7C`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            if init(me) then
                tick(me)
            end
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        me:update_sprite_while_paused()
    end,
}

-- `sub_80DEC84`: a worm of `owner`'s of level `level` on (x, y), with the
-- damage word `damage`. (It spawns at the registers' values, which its
-- init replaces but for Z.)
function worm.spawn(owner: Object, x: number, y: number, element: number, z: number, level: number, damage: number): Object?
    local o = battle.spawn(worm.kind, Vec3.new(y, element, z))
    if o then
        o.related1 = owner
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        local s = o.state :: State
        s.level = level
        s.last_x, s.last_y = x, y
        s.start_hole, s.end_hole = nil, nil
    end
    return o
end

-- `sub_80DEDE0`, instant chip effect 12: an `instant` hook that sends worms
-- of `level` behind each of the other side's navis still in, where it can.
function worm.instant(level: number): (user: Object, spec: InstantChipSpec) -> ()
    return function(user: Object, spec: InstantChipSpec)
        local side = user.alliance
        for _, target in battle.alive_actors(bit32.bxor(side, 1)) do
            local x = user.facing + target.panel_x
            local y = target.panel_y
            if field.check(x, y, REQUIRE, FORBID[side]) then
                worm.spawn(user, x, y, spec.element, spec.z, level, spec.damage)
            end
        end
    end
end

return worm
