--!strict
-- A BlkBomb (attack object #0x4A, `sub_80CD4EC`): action 0x12's subtype 6.
-- Lobbed at the panel three ahead, it sits there as an obstacle with 100
-- HP (one to a side, `obstacle.register` class 1) until a fire hit sets it
-- off: it bursts over the whole of the side opposite its panel's owner
-- (one hit, then explosions scattered over those panels), with the chip's
-- damage. Broken otherwise, it just goes. A thrown one lasts 6000 ticks;
-- one placed some other way (its first parameter set) stays. Its third
-- and fourth parameters are the panel it aims at.
--
-- An obstacle: its update runs the obstacle framework's steps (the
-- `obstacle` service) around its own actions.

local panel_bursts = require("../panel-bursts/panel_bursts")
local smoke_puff = require("../smoke-puff/smoke_puff")
local trajectory = require("../../lib/trajectory")

export type State = {
    -- The angle it was thrown at (BattleObject+0x0C).
    angle: number,
    -- A fire hit set it off (its ExtraVars word).
    set_off: boolean,
}

local black_bomb = {
    state = { angle = "u8", set_off = "bool" },
}

local SPRITE = "0c-24"
local NAME_ID = 0xD5
local HP = 0x64
local LIFETIME = 0x1770
-- Thrown at 0x2C000 a tick along the ground, falling 0x2800 a tick.
local SPEED, GRAVITY = 0x2C000, -0x2800
-- Its collision: self type 0x4F, target 0xF, hit modifier 3; it deals 100
-- with counter byte 0x1E; hit spark 0xA.
local SELF_TYPE, TARGET, HIT_MOD, SELF_DAMAGE, COUNTER, HIT_SPARK = 0x4F, 0x0F, 3, 0x64, 0x1E, 0x0A
-- Its kind among absorbed obstacles (`data.objects.absorbed_sprites`).
local ABSORBED_KIND = 1
local SOUND_LAND, SOUND_HIT_BOMB, SOUND_BURST = 0xC0, 0x70, 0xC3
-- What's left when a chip removes it (effect 0x14, 12 pixels up), and
-- when it breaks (effect 0, the explosion).
local EFFECT_REMOVED, EFFECT_BROKEN = 0x14, 0
-- The burst (`byte_80CD828`): the whole-field region of the side it hits
-- (0x82 is side 1's area, 0x81 side 0's), hit spark none, target 5, self
-- type 4, hit modifier 3, fire; then explosions there every 3 ticks
-- (panel bursts, look 1).
local BURST_REGIONS = { [0] = 0x82, [1] = 0x81 }
local BURST_TARGET, BURST_SELF_TYPE, BURST_HIT_MOD, FIRE = 0x05, 0x04, 3, 1
local BURST_INTERVAL, BURST_LOOK = 2, 1

-- The actions (the object's action byte).
local DESTROYED, SET = 2, 9

-- `sub_80CD886`: a BlkBomb thrown by `user` from `pos` at the panel three
-- ahead, with the chip's damage word: one of the side's field objects.
-- (Its parameters are only the target panel, from the user's panel as a
-- halfword; the game also keeps its thrower's X, which nothing reads.)
function black_bomb.throw(user: Object, pos: Vec3, _params: { number }, damage: number): Object?
    local xy = int.u16(user.panel_x + user.panel_y * 0x100 + 3 * user.facing)
    local o = battle.spawn_kind("black-bomb", pos, { 0, 0, bit32.band(xy, 0xFF), bit32.rshift(xy, 8) })
    if o then
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance = user.alliance
        o.run_while_dimmed = true
        obstacle.register(o, user.alliance, 1)
    end
    return o
end

-- Gone from the field's bookkeeping, and freed at the next update.
local function finish(me: Object)
    obstacle.release_tracking(me)
    obstacle.unregister(me)
    me:unreserve_panel(me.future_panel_x, me.future_panel_y)
    me:set_lifecycle("destroy")
end

-- Set down (`loc_80CD6BC`): on its panel for good.
local function set_down(me: Object)
    me.collision.region = 1
    me.run_while_dimmed = true
    me.action = SET
    me.phase, me.phase_init = 0, 0
end

-- `sub_80CD654` (actions 0 and 8): fly to the target panel; land on it,
-- or in a hole in a puff of smoke. One placed some other way sets down at
-- once.
local function fly(me: Object)
    if me:param(1) ~= 0 then
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
    local x, y = me:param(3), me:param(4)
    me.panel_x, me.panel_y = x, y
    me.collision.panel_x, me.collision.panel_y = x, y
    local cx, cy = field.center(x, y)
    me.pos = Vec3.new(cx, cy, 0)
    if field.solid(x, y) then
        battle.play_sound(SOUND_LAND)
        -- (The camera shakes, which is only drawn.)
        set_down(me)
        return
    end
    me:set_coordinates_from_panel()
    local p = me.pos
    smoke_puff.spawn(Vec3.new(p.x, p.y, 0), 0)
    me.collision.region = 0
    obstacle.unregister(me)
    obstacle.release_tracking(me)
    me.lifecycle = "destroy"
end

-- `sub_80CD700`: leaving: a chip's removal (absorbed, blinking out, or a
-- puff), else broken (an explosion, with the bomb's sound unless a fire
-- hit set it off, which bursts as it goes); held while dimmed.
local function destroyed(me: Object)
    me.collision.region = 0
    local how = obstacle.removal(me)
    if how ~= "broken" then
        if how == "absorbed" then
            obstacle.fly_to_absorber(me, ABSORBED_KIND)
        else
            local blink = obstacle.blink_out(me)
            if blink == "blinking" then
                return
            elseif blink == "no" then
                local p = me.pos
                battle.effect(Vec3.new(p.x, p.y, p.z + 0xC_0000), EFFECT_REMOVED)
            end
        end
        finish(me)
        return
    end
    if me.phase_init == 0 then
        me.phase_init = 4
        me.visible = false
        if me.hp == 0 then
            battle.effect(me.pos, EFFECT_BROKEN)
            if not (me.state :: State).set_off then
                battle.play_sound(SOUND_HIT_BOMB)
                finish(me)
                return
            end
        end
    end
    if battle.dimmed() then
        return
    end
    finish(me)
end

-- `sub_80CD6FC`: set down, nothing to do.
local function idle(_me: Object) end

-- Its action table (`off_80CD62C`): its own actions and the framework's
-- shared ones.
local ACTIONS: { [number]: (Object) -> () } = {
    [0] = fly,
    [1] = obstacle.return_to_idle,
    [2] = destroyed,
    [3] = obstacle.flinch,
    [4] = obstacle.paralyzed,
    [5] = obstacle.pushed,
    [6] = obstacle.frozen,
    [7] = obstacle.bubbled,
    [8] = fly,
    [9] = idle,
}

-- `sub_80CD7A0`: a fire hit sets it off.
local function catch_fire(me: Object)
    local s = me.state :: State
    if s.set_off or me.collision:element_damage(FIRE) == 0 then
        return
    end
    s.set_off = true
    me.collision.region = 0
    me.visible = false
    me.hp = 0
    me.action = DESTROYED
    me.phase, me.phase_init = 0, 0
end

-- `sub_80CD5F8`.
local function tick(me: Object)
    me.visible = true
    obstacle.take_hits_keeping_damage(me)
    me:hit_spark()
    catch_fire(me)
    if me:param(1) == 0 then
        obstacle.tick_lifetime(me)
    end
    local a = obstacle.react_holding(me)
    if a then
        local action = ACTIONS[a]
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
local function burst(me: Object)
    if not (me.state :: State).set_off then
        return
    end
    local panel = field.panel(me.panel_x, me.panel_y)
    if not panel then
        error(string.format("BlkBomb bursts off the field at (%d, %d)", me.panel_x, me.panel_y))
    end
    local side = bit32.bxor(panel.alliance, 1)
    me.alliance = side
    local r = BURST_REGIONS[side]
    if r == nil then
        error(string.format("BlkBomb side %d reads past byte_80CD828", side))
    end
    battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = FIRE,
        z = me.pos.z,
        region = r,
        hit_effect = 0xFF,
        target = BURST_TARGET,
        self_type = BURST_SELF_TYPE,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = BURST_HIT_MOD,
    })
    panel_bursts.spawn(me, r, BURST_INTERVAL, 0, { BURST_LOOK, 0, 0, 0 })
    -- (The camera shakes, which is only drawn.)
    battle.play_sound(SOUND_BURST)
end

-- `sub_80CD50C`: aim at the target panel (a thrown one), with 100 HP.
local function init(me: Object, s: State)
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.shadow = "ground"
    me.name_id = NAME_ID
    if me:param(1) == 0 then
        local x, y = me:param(3), me:param(4)
        me.future_panel_x, me.future_panel_y = x, y
        local tx, ty = field.center(x, y)
        local p = me.pos
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
    me:setup_collision(SELF_TYPE, TARGET, HIT_MOD)
    me.collision.self_damage = SELF_DAMAGE
    me.collision.counter_byte = COUNTER
    me.collision.region = 0
    me.collision.hit_effect = HIT_SPARK
    me.collision:present()
    me.lifecycle = "update"
    tick(me)
end

-- `sub_80CD4EC`.
function black_bomb.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me, me.state :: State)
    elseif l == "update" then
        tick(me)
    else
        -- `sub_80CD78E`.
        burst(me)
        me.collision:free()
        me:free()
    end
end

return black_bomb
