--!strict
-- A BugBomb (the original's attack object #0xA5, `sub_80D9D4C`). Lobbed at
-- the panel three ahead, it sits there with 40 HP for a second and then
-- bursts over the 3x3 around it, giving what it hits a NaviCust bug: one
-- its target's side doesn't have yet (HP drain, custom-screen HP drain, a
-- panel trail or buster blanks), picked at random. Breaking it or a
-- crushing hit first ends it in an explosion; landing in a hole, in a puff
-- of smoke.

local collision = require("../../rules/collision")
local regions = require("../../lib/regions")
local rising_bubble = require("../../objects/rising-bubble/rising_bubble")
local statuses = require("../../rules/status")
local trajectory = require("../../lib/trajectory")

local bug_bomb = {}

export type State = {
    -- The angle it was thrown at (BattleObject+0x0C).
    angle: number,
    -- The panel it aims at.
    target_x: number,
    target_y: number,
}

local SPRITE, ANIM = asset.sprite("bomb"), 5
-- Thrown at 0x2C000 a tick along the ground, to land 10 pixels up,
-- falling 0x2800 a tick.
local SPEED, LANDING_Z, GRAVITY = 0x2C000, 0x0A * 0x10000, -0x2800
local HP = 0x28
-- Collision while it sits; its status 0x20.
local HIT_MOD, STATUS = 0, statuses.confuse_480
-- The burst: ticks it sits (61), over the eight around it, and an
-- explosion (effect 0x4A) on each panel of the 3x3.
local SIT_TICKS = 0x3C
local EXPLOSION = define.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 2 }
local SOUND_LAND, SOUND_HIT_BOMB = asset.sound("bug-bomb-land"), asset.sound("hit-bomb-1")
-- What in its hits ends it at once.
local CRUSHED = 0x3F800000

-- The bugs (`byte_80DA034`): the code and its argument, by what the target
-- lacks: HP drain, custom-screen HP drain, a panel trail, buster blanks.
local BUGS = {
    [0] = { bug = 0x18, arg = 1 },
    [1] = { bug = 0x19, arg = 1 },
    [2] = { bug = 0xF5, arg = 0 },
    [3] = { bug = 0x14, arg = 4 },
}
-- The panel trail that means the target already has that bug.
local TRAIL_BUGGED = 3

-- `sub_80D9FC2`: a bug its target's side doesn't have yet, at random (one
-- RNG draw), or any of the four when it has them all.
local function pick_bug(me: Object)
    local target = battle.navi(bit32.bxor(me.alliance, 1))
    local choices = {}
    if target.hp_drain == 0 then
        table.insert(choices, 0)
    end
    if target.custom_drain == 0 then
        table.insert(choices, 1)
    end
    if target.panel_trail ~= TRAIL_BUGGED then
        table.insert(choices, 2)
    end
    if target.buster_blanks == 0 then
        table.insert(choices, 3)
    end
    if #choices == 0 then
        choices = { 0, 1, 2, 3 }
    end
    local _, i = trajectory.div(battle.rng_positive(), #choices)
    local b = BUGS[choices[i + 1]]
    me.collision.bugs = b.arg * 0x100 + b.bug
end

-- Done: hidden, off its panels, and freed at the next update.
local function finish(me: Object)
    me.collision:set_region(nil)
    me.visible = false
    me:set_lifecycle("destroy")
end

-- `sub_80D9E94`: fly until the ticks run out or it comes down; then land
-- on its target panel, 10 pixels up.
local function fly(me: Object, s: State)
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
        battle.play_sound(SOUND_LAND)
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
local function sit(me: Object)
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
    me:reset_collision_types(collision.attack, collision.hits_navis, 0)
    battle.region_effects(me.panel_x, me.panel_y, regions.square, me.alliance, EXPLOSION, 0)
    battle.shake_camera(1, 0xF)
    battle.play_sound(SOUND_HIT_BOMB)
    me.visible = false
    me.action = 8
    me.phase, me.phase_init = 0, 0
end

-- `sub_80D9E24`: resolve last tick's hits; broken or crushed, it ends in an
-- explosion; with the battle, quietly; while dimmed it stands.
local function tick(me: Object)
    me.collision:remove()
    if battle.over() then
        finish(me)
        return
    end
    if me:take_damage(0) < 0 or bit32.btest(me.collision.hit_flags, CRUSHED) then
        battle.effect(me.pos, EXPLOSION)
        battle.play_sound(SOUND_HIT_BOMB)
        finish(me)
        return
    end
    if not battle.dimmed() then
        local a = me.action
        if a == 0 then
            fly(me, me.state :: State)
        elseif a == 4 then
            sit(me)
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
local function init(me: Object, s: State)
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    me.anim, me.anim_loaded = ANIM, 0xFF
    -- (It sets the sprite's animation from the 0xFF it just stored, and
    -- steps it; the first sprite update loads the real one.)
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
    me:setup_collision(collision.own_thrown_body, collision.hit_by_other_side, HIT_MOD)
    pick_bug(me)
    me.collision.status_base = STATUS
    me.collision:set_region(nil)
    me.collision:present()
    me.lifecycle = "update"
    tick(me)
end

bug_bomb.kind = define.kind {
    id = "bugbomb/bomb",
    pool = "attack",
    state = { angle = "u8", target_x = "u8", target_y = "u8" },
    -- `sub_80D9D4C`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me, me.state :: State)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
    end,
}

-- `sub_80D9FA8`: a thrower for `throw.action` that throws a BugBomb at the
-- panel three ahead of the user. (The original keeps that panel as a
-- halfword from the user's panel: a panel off the left edge wraps.)
function bug_bomb.thrower(): Thrower
    return {
        throw = function(user: Object, pos: Vec3, damage: number): Object?
            local xy = int.u32(user.panel_x + user.panel_y * 0x100 + 3 * user.facing)
            local o = battle.spawn(bug_bomb.kind, pos)
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

return bug_bomb
