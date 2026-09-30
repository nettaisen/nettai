--!strict
-- A rolling log (attack object #0x66, `sub_80D1218`): RlngLog's. With
-- Param4 set (every RlngLog) it is thrown in an arc onto the panel ahead
-- in 13 ticks, then rolls forward at 3 pixels a tick until it leaves the
-- field; with Param4 clear it drops in from above rolling. A big one
-- (Param1 set) covers its panel and the one above it (region 0x18). It
-- has Param3 HP: a hit that takes some stops it for Param2 ticks, one
-- that takes the rest breaks it (a puff and gone 20 ticks later); a body,
-- a neutral object, the end of the battle or no solid panel under it
-- makes it burst at once. It keeps running while dimmed, but only acts
-- outside dimming.
--
-- Its first related slot can hold an object whose header it sets (to 1
-- while it rolls, 0 when it goes); RlngLog gives none.

local trajectory = require("../../lib/trajectory")

export type State = {
    -- Its HP as of its last update (ExtraVars+8).
    last_hp: number,
}

local rolling_log = {
    state = { last_hp = "u16" },
}

local SPRITE = "10-34"
local SELF_TYPE, TARGET_TYPE, HIT_MOD = 0x0C, 0x0D, 0x01
-- Its arc (Param4 set): ticks, gravity (16.16 per tick); then its speed.
local ARC_TICKS, GRAVITY, ARC_ROLL_SPEED = 0x0D, -0x8000, 0x3_0000
-- Dropping in (Param4 clear): its speed forward and down.
local DROP_ROLL_SPEED, DROP_FALL = 0x1_C000, -0x2_0000
-- Its region: one panel, or its panel and the one above (big).
local REGION, BIG_REGION = 1, 0x18
-- Effects: an explosion (0), a puff (0x14).
local EXPLOSION, PUFF = 0, 0x14
local SOUND_HIT_BOMB = 0x70
local SOLID = 0x10
-- Hits that make it burst: bodies and neutral objects.
local BURSTING = 0x0F800000
-- Ticks it lingers, hidden, after it breaks.
local BREAK_TICKS = 0x14

-- The actions.
local FLY, ROLL, STOPPED, BROKEN = 0x0, 0x4, 0x8, 0xC

-- `sub_80D1442`: a log for `owner` on (x, y) with the chip's parameters
-- and the damage word `damage`. It runs while dimmed. Its position is the
-- spawner's registers (the row, the element and a height of 16 pixels)
-- until its init places it; the height stays.
function rolling_log.spawn(owner: Object, x: number, y: number, element: number, params: { number }, damage: number): Object?
    local o = battle.spawn_kind("rolling-log", Vec3.new(y, element, 0x10_0000), params)
    if o then
        o.element = element
        o.panel_x, o.panel_y = x, y
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = nil
        o.run_while_dimmed = true
    end
    return o
end

-- A puff raised 16 pixels.
local function puff(me: Object)
    battle.effect(me.pos + Vec3.px(0, 0, 0x10), PUFF)
end

-- `sub_80D146A`: there is ground under it: a solid panel under it (or,
-- for a big one, under it or above it). Always, while it has no region.
local function grounded(me: Object): boolean
    if me.collision.region == 0 then
        return true
    end
    local x, y = me.panel_x, me.panel_y
    if bit32.btest(field.flags(x, y), SOLID) then
        return true
    end
    return me:param(1) ~= 0 and bit32.btest(field.flags(x, y - 1), SOLID)
end

-- `sub_80D149E`: thrown in an arc onto the panel ahead, or dropping in
-- from its height, rolling.
local function launch(me: Object)
    if me:param(4) ~= 0 then
        local x = me.panel_x + me.facing
        me.panel_x = x
        local cx, cy = field.center(x, me.panel_y)
        me.timer2 = ARC_TICKS
        me.vel = trajectory.in_ticks(me.pos, cx, cy, 0, ARC_TICKS, GRAVITY)
        return
    end
    local vx = me.facing * DROP_ROLL_SPEED
    me.vel = Vec3.new(vx, 0, DROP_FALL)
    local ticks = int.tdiv(me.pos.z, -DROP_FALL)
    me.timer2 = ticks
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x - int.mul32(int.u16(ticks), vx)), p.y, p.z)
end

-- ---- The actions ---------------------------------------------------------------

-- `sub_80D1394`: fly (falling faster each tick in an arc), then land on
-- its panel and roll.
local function fly(me: Object)
    local arc = me:param(4) ~= 0
    local v = me.vel
    me.pos = me.pos + v
    if arc then
        me.vel = Vec3.new(v.x, v.y, int.i32(v.z + GRAVITY))
    end
    local t = me.timer2 - 1
    me.timer2 = t
    if bit32.band(t, 0xFFFF) ~= 0 then
        return
    end
    me:set_coordinates_from_panel()
    me:update_collision_panels()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    me.collision.region = if me:param(1) ~= 0 then BIG_REGION else REGION
    me:set_action(ROLL)
    if arc then
        local w = me.vel
        me.vel = Vec3.new(me.facing * ARC_ROLL_SPEED, w.y, w.z)
    end
end

-- `sub_80D13F6`: roll forward until off the field.
local function roll(me: Object)
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x + me.vel.x), p.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if not field.valid(me.panel_x, me.panel_y) then
        me.collision.region = 0
        me:set_lifecycle("destroy")
    end
end

-- `sub_80D141A`: stopped by a hit, then roll on.
local function stopped(me: Object)
    local t = me.timer - 1
    me.timer = t
    if bit32.band(t, 0xFFFF) == 0 then
        me:set_action(ROLL)
    end
end

-- `sub_80D142C`: broken: hidden, gone after its time.
local function broken(me: Object)
    me.visible = false
    local t = me.timer - 1
    me.timer = t
    if bit32.band(t, 0xFFFF) == 0 then
        me:set_lifecycle("destroy")
    end
end

local ACTIONS: { [number]: (Object) -> () } = {
    [FLY] = fly,
    [ROLL] = roll,
    [STOPPED] = stopped,
    [BROKEN] = broken,
}

-- ---- The lifecycle -------------------------------------------------------------

-- A puff and a bang, and gone at once.
local function burst(me: Object)
    puff(me)
    battle.play_sound(SOUND_HIT_BOMB)
    me.collision.region = 0
    me:set_lifecycle("destroy")
end

-- Out of the fight: hidden, the broken action from its start.
local function to_broken(me: Object)
    me.visible = false
    me.action = BROKEN
    me.phase = 0
    me.phase_init = 0
end

-- `sub_80D12CA`: take its hits; burst, break, stop, or act (outside
-- dimming).
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() or not grounded(me) then
        return burst(me)
    end
    if me.action == BROKEN then
        to_broken(me)
    elseif me:take_damage(0) == -1 then
        battle.effect(Vec3.new(me.pos.x, me.pos.y, (me:param(1) * 0x0C + 0x10) * 0x10000), EXPLOSION)
        battle.play_sound(SOUND_HIT_BOMB)
        c.region = 0
        me.timer = BREAK_TICKS
        to_broken(me)
    elseif bit32.btest(c.hit_flags, BURSTING) then
        return burst(me)
    else
        local s = me.state :: State
        local hp = me.hp
        local last = s.last_hp
        s.last_hp = hp
        if hp ~= last then
            me.timer = me:param(2)
            me:set_action(STOPPED)
        end
    end
    if not battle.dimmed() then
        local act = ACTIONS[me.action]
        if act == nil then
            error(string.format("rolling log action %#x reads past its table", me.action))
        end
        act(me)
    end
    c:present()
end

-- `sub_80D124E`: its look, HP and path.
local function init(me: Object)
    me:set_coordinates_from_panel()
    -- sub_8011504
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite:set_animation(0)
    sprite.shadow = "with_sprite"
    me.anim = 0
    me.anim_loaded = 0xFF
    if not me:create_collision() then
        puff(me)
        me:free()
        return
    end
    me:setup_collision(SELF_TYPE, TARGET_TYPE, HIT_MOD)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.visible = true
    local big = if me:param(1) ~= 0 then 1 else 0
    me:set_param(1, big)
    me.anim = big * 2 + 1
    local c = me.collision
    c.hit_effect = 0xFF
    local hp = me:param(3)
    me.hp = hp
    me.max_hp = hp;
    (me.state :: State).last_hp = hp
    sprite.palette = 0
    c.region = 0
    c:present()
    launch(me)
    me.lifecycle = "update"
end

-- `sub_80D1218`.
function rolling_log.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
        return
    end
    if me.active then
        me:update_sprite_while_paused()
    end
end

return rolling_log
