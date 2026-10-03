--!strict
-- A BugBomb (BN6's attack object #0xA5, `sub_80D9D4C`; BN5's #0x77, 0x080D4E54, the same code but for its
-- look and its bugs). Lobbed at the panel three ahead, it sits there
-- with 40 HP for a second and then bursts over the 3x3 around it, giving what it hits a NaviCust bug its target's
-- side doesn't have yet, picked at random by its game (`look.pick_bug`). Breaking it or a crushing hit first ends
-- it in an explosion; landing in a hole, in a puff of smoke.
--
-- Each game makes its kind with `bomb.make_kind(id, look)` (BN6's chips/bugbomb, BN5's chips/bugbomb).

local regions = require("@bn6/lib/regions")
local rising_bubble = require("@bn6/objects/rising-bubble/rising_bubble")
local trajectory = require("@bn6/lib/trajectory")

local bomb = {}

export type State = {
    -- The angle it was thrown at (BattleObject+0x0C).
    angle: number,
    -- The panel it aims at.
    target_x: number,
    target_y: number,
}

-- A game's BugBomb: its sprite and animation, what it is and is hit by while it sits (and its hits' status), and
-- what it bursts as and reaches; the explosions (a crushed one's and the burst's on each panel of the 3x3), the
-- sounds of its landing, its crushing and its burst; the bug its burst gives (setting the collision's bugs from
-- its target's stats).
export type Look = {
    sprite: SpriteAsset,
    anim: number,
    is: CollisionType,
    hit_by: CollisionType,
    status: Status,
    burst_is: CollisionType,
    burst_hits: CollisionType,
    crushed: Effect,
    burst: Effect,
    land_sound: Sound,
    crushed_sound: Sound,
    burst_sound: Sound,
    pick_bug: (me: Object) -> (),
}

-- Thrown at 0x2C000 a tick along the ground, to land 10 pixels up, falling 0x2800 a tick.
local SPEED, LANDING_Z, GRAVITY = 0x2C000, 0x0A * 0x10000, -0x2800
local HP = 0x28
-- Collision while it sits.
local HIT_MOD = 0
-- The burst: ticks it sits (61).
local SIT_TICKS = 0x3C
-- What in its hits ends it at once.
local CRUSHED = 0x3F800000

-- Pick one of `choices` (indices) at random, one RNG draw, or of `all` when there are none.
function bomb.pick(choices: { number }, all: { number }): number
    if #choices == 0 then
        choices = all
    end
    local _, i = trajectory.div(battle.rng_positive(), #choices)
    return choices[i + 1]
end

-- Done: hidden, off its panels, and freed at the next update.
local function finish(me: Object)
    me.collision:set_region(nil)
    me.visible = false
    me:set_lifecycle("destroy")
end

-- `sub_80D9E94`: fly until the ticks run out or it comes down; then land on its target panel, 10 pixels up.
local function fly(me: Object, look: Look, s: State)
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        local v = me.vel
        local vz = int.i32(v.z + GRAVITY)
        me.vel = Vec3.new(v.x, v.y, vz)
        local p = me.pos
        local moved = Vec3.new(p.x + v.x, p.y + v.y, p.z + vz)
        if moved.z > 0 then
            me.pos = moved
            return
        end
    end
    local x, y = s.target_x, s.target_y
    me.panel_x, me.panel_y = x, y
    me.collision.panel_x, me.collision.panel_y = x, y
    local cx, cy = field.center(x, y)
    me.pos = Vec3.new(cx, cy, LANDING_Z)
    if field.solid(x, y) then
        me.collision:set_region(regions.single)
        me.run_while_dimmed = true
        battle.play_sound(look.land_sound)
        me.action = 4
        me.phase, me.phase_init = 0, 0
        return
    end
    me:set_coordinates_from_panel()
    local p = me.pos
    rising_bubble.spawn(Vec3.new(p.x, p.y, 0), 0)
    me.collision:set_region(nil)
    me.visible = false
    me.lifecycle = "destroy"
end

-- `sub_80D9F2C`: sit; then burst.
local function sit(me: Object, look: Look)
    if me.phase_init == 0 then
        me.timer = SIT_TICKS
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t >= 0 then
        return
    end
    me.collision:set_region(regions.ring)
    me:reset_collision_types(look.burst_is, look.burst_hits, 0)
    battle.region_effects(me.panel_x, me.panel_y, regions.square, me.alliance, look.burst, 0)
    battle.shake_camera(1, 0xF)
    battle.play_sound(look.burst_sound)
    me.visible = false
    me.action = 8
    me.phase, me.phase_init = 0, 0
end

-- `sub_80D9E24`: resolve last tick's hits; broken or crushed, it ends in an explosion; with the battle, quietly;
-- while dimmed it stands.
local function tick(me: Object, look: Look)
    me.collision:remove()
    if battle.over() then
        finish(me)
        return
    end
    if me:take_damage(0) < 0 or bit32.btest(me.collision.hit_flags, CRUSHED) then
        battle.effect(me.pos, look.crushed)
        battle.play_sound(look.crushed_sound)
        finish(me)
        return
    end
    if not battle.dimmed() then
        local a = me.action
        if a == 0 then
            fly(me, look, me.state :: State)
        elseif a == 4 then
            sit(me, look)
        elseif a == 8 then
            -- `sub_80D9F84`: burst; gone.
            me.collision:set_region(nil)
            me:set_lifecycle("destroy")
        else
            error(string.format("BugBomb action %#x reads past its table", a))
        end
        me:update_sprite()
    end
    me.collision:present()
end

-- `sub_80D9D6C`: aim at the target panel and set out.
local function init(me: Object, look: Look, s: State)
    me.visible = true
    local sprite = me.sprite
    sprite:load(look.sprite)
    me.anim, me.anim_loaded = look.anim, 0xFF
    -- (It sets the sprite's animation from the 0xFF it just stored, and steps it; the first sprite update loads
    -- the real one.)
    sprite:set_animation(0xFF)
    sprite:step()
    sprite.shadow = "ground"
    me.future_panel_x, me.future_panel_y = s.target_x, s.target_y
    local tx, ty = field.center(s.target_x, s.target_y)
    local p = me.pos
    s.angle = trajectory.angle(int.i32(tx - p.x), int.i32(ty - p.y))
    local vx, vy = trajectory.toward(s.angle, SPEED)
    me.vel = Vec3.new(vx, vy, me.vel.z)
    local vz, ticks = trajectory.at_speed(me.pos, vy, tx, ty, LANDING_Z, SPEED, GRAVITY)
    me.vel = Vec3.new(vx, vy, vz)
    me.timer = ticks
    me.hp, me.max_hp = HP, HP
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(look.is, look.hit_by, HIT_MOD)
    look.pick_bug(me)
    me.collision.status_base = look.status
    me.collision:set_region(nil)
    me.collision:present()
    me.lifecycle = "update"
    tick(me, look)
end

-- A game's BugBomb kind `id`, of `look`.
function bomb.make_kind(id: string, look: Look): Kind
    return define.kind {
        id = id,
        pool = "attack",
        state = { angle = "u8", target_x = "u8", target_y = "u8" },
        -- `sub_80D9D4C`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me, look, me.state :: State)
            elseif l == "update" then
                tick(me, look)
            else
                me:destroy()
            end
        end,
    }
end

-- `sub_80D9FA8`: a thrower for `throw.action` that throws a BugBomb of `kind` at the panel three ahead of the
-- user. (The original keeps that panel as a halfword from the user's panel: a panel off the left edge wraps.)
function bomb.thrower(kind: Kind): Thrower
    return {
        throw = function(user: Object, pos: Vec3, damage: number): Object?
            local xy = int.u32(user.panel_x + user.panel_y * 0x100 + 3 * user.facing)
            local o = battle.spawn(kind, pos)
            if o then
                local s = o.state :: State
                s.target_x = bit32.band(xy, 0xFF)
                s.target_y = bit32.extract(xy, 8, 8)
                o:set_damage_word(damage)
                o.alliance, o.flip = user.alliance, user.flip
            end
            return o
        end,
    }
end

return bomb
