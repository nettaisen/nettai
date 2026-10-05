--!strict
-- A BlkBomb, as EXE5 and EXE6 share it (EXE6's attack object #0x4A,
-- `sub_80CD4EC`; EXE5's #0x47). Lobbed at
-- the panel three ahead, it sits there as an obstacle with 100 HP (one to a
-- side, `obstacle.register` class 1) until a fire hit sets it off: it
-- bursts over the whole of the side opposite its panel's owner (one hit,
-- then explosions scattered over those panels), with the chip's damage.
-- Broken otherwise, it just goes. A thrown one lasts 6000 ticks; one placed
-- some other way (`placed`) stays.
--
-- An obstacle: its update runs the obstacle framework's steps (the
-- `obstacle` service) around its own actions. Each game's bomb is a kind of
-- its look (`black_bomb.make_kind`): how it leaves the field, its identity,
-- sprite, hit spark, sounds, its and its burst's collision types, how a
-- pushing hit treats it, and the objects and helpers it uses (the game's
-- rising bubble, panel bursts and trajectory).

local panel_bursts = require("../panel-bursts")
local regions = require("../regions")
local rising_bubble = require("../rising-bubble")

local black_bomb = {}

-- A game's BlkBomb: its destroyed action (2), identity (a field object,
-- with the look something that takes its place shows: DustMan's junk) and
-- sprite, hit spark, collision type, what it hits and its burst's, what a
-- pushing hit does to it, the sounds of its landing and its burst, the
-- burst's explosions, the bubble it leaves in a hole, its panel bursts'
-- kind and its trajectory.
export type Look = {
    destroyed: (Object) -> (),
    identity: Identity,
    sprite: SpriteAsset,
    spark: Spark,
    is: CollisionType,
    target: CollisionType,
    burst_is: CollisionType,
    burst_target: CollisionType,
    push: ObstaclePush,
    land_sound: Sound,
    burst_sound: Sound,
    burst_effect: Effect,
    bubble: Kind,
    bursts: Kind,
    trajectory: Trajectory,
}

-- A kind's look and its action table.
type Kinded = Look & { actions: { [number]: (Object) -> () } }

export type State = {
    -- The angle it was thrown at (BattleObject+0x0C).
    angle: number,
    -- A fire hit set it off (its ExtraVars word).
    set_off: boolean,
    -- Placed some other way than thrown: it sets down at once and stays
    -- (the original's first parameter).
    placed: boolean,
    -- The panel it aims at.
    target_x: number,
    target_y: number,
}

local HP = 0x64
local LIFETIME = 0x1770
-- Thrown at 0x2C000 a tick along the ground, falling 0x2800 a tick.
local SPEED, GRAVITY = 0x2C000, -0x2800
-- Its collision: hit modifier 3; it deals 100 with counter byte 0x1E.
local HIT_MOD, SELF_DAMAGE, COUNTER = 3, 0x64, 0x1E
-- The burst (`byte_80CD828`): over the whole area of the side it hits, no
-- hit spark, hit modifier 3, fire; then explosions there every 3 ticks
-- (panel bursts).
local BURST_HIT_MOD, FIRE = 3, 1
local BURST_INTERVAL = 2

-- The actions (the object's action byte).
local DESTROYED, SET = 2, 9

-- Gone from the field's bookkeeping, and freed at the next update.
function black_bomb.finish(me: Object)
    obstacle.release_tracking(me)
    obstacle.unregister(me)
    me:unreserve_panel(me.future_panel_x, me.future_panel_y)
    me:set_lifecycle("destroy")
end

-- Set down (`loc_80CD6BC`): on its panel for good.
local function set_down(me: Object)
    me.collision:set_region(regions.single)
    me.run_while_dimmed = true
    me.action = obstacle.action_byte(me, SET)
    me.phase, me.phase_init = 0, 0
end

-- `sub_80CD654` (actions 0 and 8): fly to the target panel; land on it,
-- or in a hole in a puff of smoke. One placed some other way sets down at
-- once.
local function fly(me: Object, look: Look)
    local s = me.state :: State
    if s.placed then
        set_down(me)
        return
    end
    local t = me.timer - 1
    me.timer = t
    if t ~= 0 then
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
    me.pos = Vec3.new(cx, cy, 0)
    if field.solid(x, y) then
        battle.play_sound(look.land_sound)
        battle.shake_camera(1, 0xF)
        set_down(me)
        return
    end
    me:set_coordinates_from_panel()
    local p = me.pos
    rising_bubble.spawn(look.bubble, Vec3.new(p.x, p.y, 0), 0)
    me.collision:set_region(nil)
    obstacle.unregister(me)
    obstacle.release_tracking(me)
    me.lifecycle = "destroy"
end

-- `sub_80CD6FC`: set down, nothing to do.
local function idle(_me: Object) end

-- `sub_80CD7A0`: a fire hit sets it off.
local function catch_fire(me: Object)
    local s = me.state :: State
    if s.set_off or me.collision:element_damage(FIRE) == 0 then
        return
    end
    s.set_off = true
    me.collision:set_region(nil)
    me.visible = false
    me.hp = 0
    me.action = DESTROYED
    me.phase, me.phase_init = 0, 0
end

-- `sub_80CD5F8`, with the kind's action table (`off_80CD62C`).
local function tick(me: Object, k: Kinded)
    me.visible = true
    obstacle.take_hits(me, k.push)
    me:hit_spark()
    catch_fire(me)
    if not (me.state :: State).placed then
        obstacle.tick_lifetime(me)
    end
    local a = obstacle.react(me, "breaks", "always")
    if a then
        local action = k.actions[a]
        if not action then
            error(string.format("BlkBomb action %d reads past off_80CD62C", a))
        end
        action(me)
    end
    me:update_sprite()
    me.collision:present()
end

-- `sub_80CD7D0`: set off, it bursts over the side opposite its panel's
-- owner.
local function burst(me: Object, k: Kinded)
    if not (me.state :: State).set_off then
        return
    end
    local panel = field.panel(me.panel_x, me.panel_y)
    if not panel then
        error(string.format("BlkBomb bursts off the field at (%d, %d)", me.panel_x, me.panel_y))
    end
    local side = bit32.bxor(panel.alliance, 1)
    me.alliance = side
    -- The area of the side it hits: the other side's.
    local area = regions.area[bit32.bxor(side, 1)]
    battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = FIRE,
        z = me.pos.z,
        region = area,
        -- None.
        hit_effect = nil,
        target = k.burst_target,
        self_type = k.burst_is,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = BURST_HIT_MOD,
    })
    panel_bursts.spawn(k.bursts, me, area, BURST_INTERVAL, 0, k.burst_effect)
    battle.shake_camera(1, 0x3C)
    battle.play_sound(k.burst_sound)
end

-- `sub_80CD50C`: aim at the target panel (a thrown one), with 100 HP.
local function init(me: Object, s: State, k: Kinded)
    me.visible = true
    local sprite = me.sprite
    sprite:load(k.sprite)
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.shadow = "ground"
    me.identity = k.identity
    if not s.placed then
        local x, y = s.target_x, s.target_y
        me.future_panel_x, me.future_panel_y = x, y
        local tx, ty = field.center(x, y)
        local p = me.pos
        local trajectory = k.trajectory
        s.angle = trajectory.angle(int.i32(tx - p.x), int.i32(ty - p.y))
        local vx, vy = trajectory.toward(s.angle, SPEED)
        me.vel = Vec3.new(vx, vy, me.vel.z)
        local vz, ticks = trajectory.at_speed(me.pos, vy, tx, ty, 0, SPEED, GRAVITY)
        me.vel = Vec3.new(vx, vy, vz)
        me.timer = ticks
        if ticks == 0 then
            me.pos = Vec3.new(tx, ty, 0)
            obstacle.release_tracking(me)
            obstacle.unregister(me)
            me:destroy()
            return
        end
    end
    me.hp, me.max_hp = HP, HP
    me.timer = LIFETIME
    if not me:create_collision() then
        obstacle.release_tracking(me)
        obstacle.unregister(me)
        me:free()
        return
    end
    me:setup_collision(k.is, k.target, HIT_MOD)
    me.collision.self_damage = SELF_DAMAGE
    me.collision.counter_byte = COUNTER
    me.collision:set_region(nil)
    me.collision:set_hit_effect(k.spark)
    me.collision:present()
    me.lifecycle = "update"
    tick(me, k)
end

-- A BlkBomb kind (`id`) of `look`.
function black_bomb.make_kind(id: string, look: Look): Kind
    -- Its action table (`off_80CD62C`): its own actions and the
    -- framework's shared ones.
    local function flying(me: Object)
        fly(me, look)
    end
    local actions: { [number]: (Object) -> () } = {
        [0] = flying,
        [1] = obstacle.return_to_idle,
        [2] = look.destroyed,
        [3] = obstacle.flinch,
        [4] = obstacle.paralyzed,
        [5] = obstacle.knocked_back,
        [6] = obstacle.frozen,
        [7] = obstacle.bubbled,
        [8] = flying,
        [9] = idle,
    }
    local k: Kinded = {
        destroyed = look.destroyed,
        identity = look.identity,
        sprite = look.sprite,
        spark = look.spark,
        is = look.is,
        target = look.target,
        burst_is = look.burst_is,
        burst_target = look.burst_target,
        push = look.push,
        land_sound = look.land_sound,
        burst_sound = look.burst_sound,
        burst_effect = look.burst_effect,
        bubble = look.bubble,
        bursts = look.bursts,
        trajectory = look.trajectory,
        actions = actions,
    }
    return define.kind {
        id = id,
        pool = "attack",
        state = { angle = "u8", set_off = "bool", placed = "bool", target_x = "u8", target_y = "u8" },
        -- `sub_80CD4EC`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me, me.state :: State, k)
            elseif l == "update" then
                tick(me, k)
            else
                -- `sub_80CD78E`.
                burst(me, k)
                me.collision:free()
                me:free()
            end
        end,
    }
end

-- `sub_80CD886`: a thrower for `throw.action` that throws a BlkBomb at the
-- panel three ahead of the user: one of the side's field objects. (The
-- original keeps that panel as a halfword from the user's panel, and its
-- thrower's X, which nothing reads.)
-- (Of `kind`, a game's.)
function black_bomb.thrower(bomb_kind: Kind): Thrower
    return {
        throw = function(user: Object, pos: Vec3, damage: number): Object?
            local xy = int.u16(user.panel_x + user.panel_y * 0x100 + 3 * user.facing)
            local o = battle.spawn(bomb_kind, pos)
            if o then
                local s = o.state :: State
                s.target_x = bit32.band(xy, 0xFF)
                s.target_y = bit32.rshift(xy, 8)
                o:set_damage_word(damage)
                o.alliance = user.alliance
                o.run_while_dimmed = true
                obstacle.register(o, user.alliance, 1)
            end
            return o
        end,
    }
end

return black_bomb
