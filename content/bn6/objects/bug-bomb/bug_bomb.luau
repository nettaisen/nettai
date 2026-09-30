--!strict
-- A BugBomb (attack object #0xA5, `sub_80D9D4C`): action 0x12's subtype 7.
-- Lobbed at the panel three ahead, it sits there with 40 HP for a second
-- and then bursts over the 3x3 around it, giving what it hits a NaviCust
-- bug: one its target's side doesn't have yet (HP drain, custom-screen HP
-- drain, a panel trail or buster blanks), picked at random. Breaking it
-- or a crushing hit first ends it in an explosion; landing in a hole, in
-- a puff of smoke.

local region = require("../../lib/region")
local smoke_puff = require("../smoke-puff/smoke_puff")
local trajectory = require("../../lib/trajectory")

export type State = {
    -- The angle it was thrown at (BattleObject+0x0C).
    angle: number,
}

local bug_bomb = {
    state = { angle = "u8" },
}

local SPRITE, ANIM = "0c-02", 5
-- Thrown at 0x2C000 a tick along the ground, to land 10 pixels up,
-- falling 0x2800 a tick.
local SPEED, LANDING_Z, GRAVITY = 0x2C000, 0x0A * 0x10000, -0x2800
local HP = 0x28
-- Collision while it sits: self type 0x4E, target 0x14; its status 0x20.
local SELF_TYPE, TARGET, HIT_MOD, STATUS = 0x4E, 0x14, 0, 0x20
-- The burst: ticks it sits (61), its region (0x10) with self type 4
-- and target 5, and an explosion (effect 0x4A) on each panel of the 3x3.
local SIT_TICKS = 0x3C
local BURST_REGION, BURST_SELF_TYPE, BURST_TARGET = 0x10, 0x04, 0x05
local EFFECT_BURST, EFFECTS_REGION = 0x4A, 0x0F
local SOUND_LAND, SOUND_HIT_BOMB = 0x115, 0x70
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

-- `sub_80D9FA8`: a BugBomb thrown by `user` from `pos` at the panel three
-- ahead, with the chip's damage word. (Its parameters are only the target
-- panel, from the user's panel as a halfword.)
function bug_bomb.throw(user: Object, pos: Vec3, _params: { number }, damage: number): Object?
    local xy = user.panel_x + user.panel_y * 0x100 + 3 * user.facing
    local o = battle.spawn_kind("bug-bomb", pos, { 0, 0, bit32.band(xy, 0xFF), bit32.extract(int.u32(xy), 8, 8) })
    if o then
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = user.alliance, user.flip
    end
    return o
end

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
    me.collision.region = 0
    me.visible = false
    me:set_lifecycle("destroy")
end

-- `sub_80D9E94`: fly until the ticks run out or it comes down; then land
-- on its target panel, 10 pixels up.
local function fly(me: Object)
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
    local x, y = me:param(3), me:param(4)
    me.panel_x, me.panel_y = x, y
    me.collision.panel_x, me.collision.panel_y = x, y
    local cx, cy = field.center(x, y)
    me.pos = Vec3.new(cx, cy, LANDING_Z)
    if field.solid(x, y) then
        me.collision.region = 1
        me.run_while_dimmed = true
        battle.play_sound(SOUND_LAND)
        me.action = 4
        me.phase, me.phase_init = 0, 0
        return
    end
    me:set_coordinates_from_panel()
    local p = me.pos
    smoke_puff.spawn(Vec3.new(p.x, p.y, 0), 0)
    me.collision.region = 0
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
    me.collision.region = BURST_REGION
    me:reset_collision_types(BURST_SELF_TYPE, BURST_TARGET, 0)
    region.effects(me.panel_x, me.panel_y, EFFECT_BURST, EFFECTS_REGION, region.direction(me.alliance), 0)
    -- (The camera shakes, which draws the local RNG only.)
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
        battle.effect(me.pos, EFFECT_BURST)
        battle.play_sound(SOUND_HIT_BOMB)
        finish(me)
        return
    end
    if not battle.dimmed() then
        local a = me.action
        if a == 0 then
            fly(me)
        elseif a == 4 then
            sit(me)
        elseif a == 8 then
            -- `sub_80D9F84`: burst; gone.
            me.collision.region = 0
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
    me.future_panel_x, me.future_panel_y = me:param(3), me:param(4)
    local tx, ty = field.center(me:param(3), me:param(4))
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
    me:setup_collision(SELF_TYPE, TARGET, HIT_MOD)
    pick_bug(me)
    me.collision.status_base = STATUS
    me.collision.region = 0
    me.collision:present()
    me.lifecycle = "update"
    tick(me)
end

-- `sub_80D9D4C`.
function bug_bomb.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me, me.state :: State)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
end

return bug_bomb
