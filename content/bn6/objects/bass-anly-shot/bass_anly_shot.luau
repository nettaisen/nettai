--!strict
-- BassAnly's shot (attack object #0xC1, `sub_80DD34C`): a dark ball that
-- starts 24 pixels ahead, kicks back and accelerates forward along a
-- slant (Param2: -1 up, 1 down; Param1 picks its speeds, 3 twice as
-- fast), its body a hit region (hit, it goes off until it reaches another
-- panel). After its first stretch (32 ticks, 16 for Param1 3) it flies
-- straight ahead, turning (Param3, its homing: 1 toward the enemy navi on
-- a diagonal, 2 toward an enemy navi one or two panels up or down the
-- slant ahead, picked at random) to run diagonally, a panel every few
-- ticks, after crossing its panel's centre (Param4 the ticks to the
-- first). It bursts where the floor ends or it leaves the field. With
-- homing 2 it leaves an afterimage every 6 ticks. It runs while dimmed.
-- See docs/engine/chips.md §3.6.35.

local bass_anly_shot = {}

export type State = {
    -- The panel it last hit something on (y << 8 | x, ExtraVars[0]).
    hit_panel: number,
    -- Its homing at spawn (Param3, ExtraVars[1]): 2 leaves afterimages.
    trail: number,
    -- Ticks since its last afterimage (ExtraVars[2]).
    trail_ticks: number,
}

bass_anly_shot.state = { hit_panel = "u32", trail = "u32", trail_ticks = "u32" }

local SPRITE = "10-0D"

-- Its first stretch: the kick back, and the push forward a tick while it
-- lasts (16.16).
local KICK = 0x3_8000
local PUSH = 0x4000
local SLANT = 0xC000

-- By Param1, its speed up or down the slant and forward once turned
-- (`byte_80DD6A8`, `byte_80DD6D0`).
local TURN_Y = { [0] = 0x2_6666, 0x2_B333, 0x3_0000, 0x7_8000 }
local TURN_X = { [0] = 0x3_8000, 0x4_8000, 0x5_0000, 0xC_8000 }

-- What homing 2 looks for, by side (`off_80DD6BC`): an enemy navi's body;
-- and where, along the slant (`byte_80DD6E8` going down, `byte_80DD6ED`
-- going up).
local ENEMY_NAVI = { [0] = 0x0400_0000, 0x0800_0000 }
local LOOK_DOWN = { { 1, -1 }, { 2, -2 } }
local LOOK_UP = { { 1, 1 }, { 2, 2 } }

-- What a hit must include to take its region off (as one word).
local HIT_MASK = 0x1E80_0000

-- Its afterimages: every 6 ticks, 12 ticks each, tinted.
local TRAIL_EVERY, TRAIL_TICKS, TRAIL_SHADER = 6, 0xC, 0xA94A

-- `sub_80DD594` (+ `sub_80DD5B4`: it runs while dimmed): a shot from `owner`
-- on (x, y) with `element`, the damage word and `params` {speeds, slant
-- (signed byte), homing, turn}. `z` is the spawner's register that
-- becomes its Z.
function bass_anly_shot.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    params: { number },
    damage: number,
    z: number
): Object?
    local o = battle.spawn_kind("bass-anly-shot", Vec3.new(y, element, z), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
        o.run_while_dimmed = true
    end
    return o
end

local function signed_byte(v: number): number
    return if v >= 0x80 then v - 0x100 else v
end

-- `sub_80DD382`: 24 pixels ahead of its panel, kicking back; collision
-- on (no hit spark).
local function init(me: Object)
    me:set_coordinates_from_panel()
    local f = me.facing
    local p = me.pos
    me.pos = Vec3.new(p.x + f * 0x18_0000, p.y, p.z)
    local fast = me:param(1) == 3
    local vy = int.mul32(SLANT, signed_byte(me:param(2)))
    if fast then
        vy = int.shl(vy, 1)
    end
    me.vel = Vec3.new(int.mul32(KICK * f, -1), vy, me.vel.z)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(0xA, 5, 3)
    me.collision.hit_effect = 0xFF
    me.collision:present()
    local s = me.state :: State
    s.trail = me:param(3)
    me.timer = if fast then 0x10 else 0x20
    me.timer2 = 0x1C + me:param(1) * 2
    battle.play_sound(0x140)
    me:set_lifecycle("update")
end

-- `sub_80DD584`: its region off, bursting (animation 1).
local function burst(me: Object)
    me.collision.region = 0
    me.anim = 1
    me:set_lifecycle("destroy")
end

-- `sub_80DD6F4`: an afterimage every 6 ticks (homing 2).
local function trail(me: Object)
    local s = me.state :: State
    if s.trail ~= 2 then
        return
    end
    local n = s.trail_ticks + 1
    if n >= TRAIL_EVERY then
        battle.afterimage(me, me.pos, {
            sprite = SPRITE,
            anim = me.anim,
            flip = bit32.bxor(me.alliance, me.flip),
            color_shader = TRAIL_SHADER,
            lifetime = TRAIL_TICKS,
        })
        n = 0
    end
    s.trail_ticks = n
end

-- `sub_80DD738`: pushed forward while Timer2 lasts.
local function push(me: Object)
    local left = me.timer2
    if left == 0 then
        return
    end
    me.timer2 = left - 1
    local push = PUSH
    if me:param(1) == 3 then
        push *= 2
    end
    local v = me.vel
    me.vel = Vec3.new(v.x + push * me.facing, v.y, v.z)
end

-- `sub_80DD5C8`: bursts where the floor ends.
local function check_floor(me: Object)
    if not field.solid(me.panel_x, me.panel_y) then
        me:set_action(0xC)
    end
end

-- A speed by Param1.
local function speed(t: { [number]: number }, p1: number, name: string): number
    local v = t[p1]
    if v == nil then
        error(string.format("BassAnly's shot %d reads past %s", p1, name))
    end
    return v
end

-- The turn: up or down the slant at `vy` and forward, a panel every few
-- ticks after the next centre (Param4 5).
local function turn(me: Object, vy: number)
    me.vel = Vec3.new(int.mul32(me.facing, speed(TURN_X, me:param(1), "byte_80DD6D0")), vy, me.vel.z)
    me:set_param(4, 5)
end

-- `sub_80DD5E2`: its homing.
local function home(me: Object)
    local mode = me:param(3)
    if mode == 0 then
        return
    end
    local x, y = me.panel_x, me.panel_y
    local p1 = me:param(1)
    if mode ~= 1 then
        if battle.over() then
            -- On its own side's panels it keeps going.
            if bit32.band(field.flags(x, y), 0x20) == me.alliance * 0x20 then
                return
            end
        else
            -- `sub_8015D80`: an enemy navi on one of the panels (one draw to
            -- pick among them).
            local look = if me:param(2) == 1 then LOOK_DOWN else LOOK_UP
            local dir = 1 - 2 * me.alliance
            local found = 0
            for _, o in look do
                if field.check(x + o[1] * dir, y + o[2], ENEMY_NAVI[me.alliance], 0) then
                    found += 1
                end
            end
            if found == 0 then
                return
            end
            battle.rng_positive()
        end
        turn(me, int.mul32(speed(TURN_Y, p1, "byte_80DD6A8"), -signed_byte(me:param(2))))
        me:set_param(3, 0)
        return
    end
    -- Homing 1: toward the enemy navi on a diagonal, once level with or
    -- past its column.
    if bit32.band(field.flags(x, y), 0x20) ~= 0 then
        return
    end
    local ex, ey = x, y
    local enemy = battle.player(bit32.bxor(me.alliance, 1))
    if enemy then
        ex, ey = enemy.panel_x, enemy.panel_y
    end
    if ex > x then
        return
    end
    if ey ~= y then
        if math.abs(ex - x) ~= math.abs(ey - y) then
            return
        end
        local vy = speed(TURN_Y, p1, "byte_80DD6A8")
        if y ~= 1 then
            vy = -vy
        end
        turn(me, vy)
    end
    me:set_param(3, 0)
end

-- `sub_80DD4B0`: the first stretch, along the slant.
local function start(me: Object)
    trail(me)
    local p, v = me.pos, me.vel
    me.pos = Vec3.new(p.x + v.x, p.y + v.y, p.z)
    push(me)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    local left = me.timer - 1
    me.timer = left
    if left == 0 then
        me:set_action(4)
    end
    check_floor(me)
end

-- `sub_80DD4E8`: straight ahead; after a panel's centre with a turn
-- pending (Param4), the diagonal run; else the floor and its homing.
local function ahead(me: Object)
    trail(me)
    local p, v = me.pos, me.vel
    local old_x = p.x
    me.pos = Vec3.new(p.x + v.x, p.y, p.z)
    push(me)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    local cx = field.center(me.panel_x, me.panel_y)
    local new_x = me.pos.x
    local passed = if new_x > old_x then cx > old_x and cx <= new_x else cx > new_x and cx <= old_x
    local turn_ticks = me:param(4)
    if passed and turn_ticks ~= 0 then
        me.timer = turn_ticks
        me:set_action(8)
        return
    end
    check_floor(me)
    home(me)
end

-- `sub_80DD538`: the diagonal run, a panel forward and up or down every
-- 10 - 2 * Param1 ticks.
local function diagonal(me: Object)
    trail(me)
    local p, v = me.pos, me.vel
    me.pos = Vec3.new(p.x + v.x, p.y + v.y, p.z)
    local left = me.timer - 1
    me.timer = left
    if left == 0 then
        me.panel_x += me.facing
        me.panel_y += if v.y < 0 then -1 else 1
        me:update_collision_panels()
        me.timer = 0xA - 2 * me:param(1)
    end
    check_floor(me)
end

local ACTIONS = { [0] = start, [4] = ahead, [8] = diagonal, [0xC] = burst }

-- `sub_80DD438`: resolve the hits (a hit takes its region off until it
-- reaches another panel), move; register again while on the field.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if not battle.dimmed() and battle.over() then
        return burst(me)
    end
    local s = me.state :: State
    if bit32.band(c.hit_flags, HIT_MASK) ~= 0 then
        c.region = 0
        s.hit_panel = bit32.bor(me.panel_x, bit32.lshift(me.panel_y, 8))
    end
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("BassAnly's shot action %#x reads past its table (off_80DD4A0)", me.action))
    end
    action(me)
    if me.lifecycle ~= "destroy" and c.region == 0 and bit32.bor(me.panel_x, bit32.lshift(me.panel_y, 8)) ~= s.hit_panel then
        c.region = 1
    end
    if field.valid(me.panel_x, me.panel_y) then
        c:present()
    else
        burst(me)
    end
end

-- `sub_80DD370`: gone once the burst has played.
local function bursting(me: Object)
    if bit32.band(me.sprite.frame_flags, 0x80) ~= 0 then
        me:destroy()
    end
end

-- `sub_80DD34C`.
function bass_anly_shot.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        bursting(me)
    end
    me:update_sprite()
end

return bass_anly_shot
