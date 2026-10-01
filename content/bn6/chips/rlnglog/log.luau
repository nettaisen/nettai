--!strict
-- A rolling log (attack object #0x66, `sub_80D1218`): RlngLog's. Thrown
-- (every RlngLog's) it flies in an arc onto the panel ahead in 13 ticks,
-- then rolls forward at 3 pixels a tick until it leaves the field; not
-- thrown it drops in from above rolling. A big one covers its panel and
-- the one above it. It has HP: a hit that takes some stops it for a while,
-- one that takes the rest breaks it (a puff and gone 20 ticks later); a
-- body, a neutral object, the end of the battle or no solid panel under it
-- makes it burst at once. It keeps running while dimmed, but only acts
-- outside dimming.
--
-- What a log is (`RollingLog`, types.d.luau) was the original's four
-- parameters: big, its stop's ticks, its HP, thrown. Its first related
-- slot can hold an object whose header it sets (to 1 while it rolls, 0
-- when it goes); RlngLog gives none.

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local regions = require("../../lib/regions")
local trajectory = require("../../lib/trajectory")

local log = {}

export type State = {
    -- Its HP as of its last update (ExtraVars+8).
    last_hp: number,
    -- What it is (its spawner's log).
    big: boolean,
    stop_ticks: number,
    thrown: boolean,
}

local SPRITE = asset.sprite("rolling-log")
local HIT_MOD = 0x01
-- Its arc (thrown): ticks, gravity (16.16 per tick); then its speed.
local ARC_TICKS, GRAVITY, ARC_ROLL_SPEED = 0x0D, -0x8000, 0x3_0000
-- Dropping in: its speed forward and down.
local DROP_ROLL_SPEED, DROP_FALL = 0x1_C000, -0x2_0000
local SOUND_HIT_BOMB = asset.sound("hit-bomb-1")
local SOLID = 0x10
-- Hits that make it burst: bodies and neutral objects.
local BURSTING = 0x0F800000
-- Ticks it lingers, hidden, after it breaks.
local BREAK_TICKS = 0x14
-- How high it is drawn, and its explosion: 16 pixels up, 12 more for a
-- big one.
local HEIGHT, BIG_HEIGHT = 0x10, 0x0C

-- The actions.
local FLY, ROLL, STOPPED, BROKEN = 0x0, 0x4, 0x8, 0xC

-- A puff raised 16 pixels.
local function puff(me: Object)
    battle.effect(me.pos + Vec3.px(0, 0, HEIGHT), effects.puff)
end

-- Its region: one panel, or its panel and the one above (big).
local function region(s: State): Region
    return if s.big then regions.tall else regions.single
end

-- `sub_80D146A`: there is ground under it: a solid panel under it (or,
-- for a big one, under it or above it). Always, while it has no region.
local function grounded(me: Object, s: State): boolean
    if me.collision.region == 0 then
        return true
    end
    local x, y = me.panel_x, me.panel_y
    if bit32.btest(field.flags(x, y), SOLID) then
        return true
    end
    return s.big and bit32.btest(field.flags(x, y - 1), SOLID)
end

-- `sub_80D149E`: thrown in an arc onto the panel ahead, or dropping in
-- from its height, rolling.
local function launch(me: Object, s: State)
    if s.thrown then
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
local function fly(me: Object, s: State)
    local v = me.vel
    me.pos = me.pos + v
    if s.thrown then
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
    me.collision:set_region(region(s))
    me:set_action(ROLL)
    if s.thrown then
        local w = me.vel
        me.vel = Vec3.new(me.facing * ARC_ROLL_SPEED, w.y, w.z)
    end
end

-- `sub_80D13F6`: roll forward until off the field.
local function roll(me: Object, _: State)
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x + me.vel.x), p.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if not field.valid(me.panel_x, me.panel_y) then
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
    end
end

-- `sub_80D141A`: stopped by a hit, then roll on.
local function stopped(me: Object, _: State)
    local t = me.timer - 1
    me.timer = t
    if bit32.band(t, 0xFFFF) == 0 then
        me:set_action(ROLL)
    end
end

-- `sub_80D142C`: broken: hidden, gone after its time.
local function broken(me: Object, _: State)
    me.visible = false
    local t = me.timer - 1
    me.timer = t
    if bit32.band(t, 0xFFFF) == 0 then
        me:set_lifecycle("destroy")
    end
end

local ACTIONS: { [number]: (Object, State) -> () } = {
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
    me.collision:set_region(nil)
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
    local s = me.state :: State
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() or not grounded(me, s) then
        return burst(me)
    end
    if me.action == BROKEN then
        to_broken(me)
    elseif me:take_damage(0) == -1 then
        local lift = if s.big then HEIGHT + BIG_HEIGHT else HEIGHT
        battle.effect(Vec3.new(me.pos.x, me.pos.y, lift * 0x10000), effects.explosion)
        battle.play_sound(SOUND_HIT_BOMB)
        c:set_region(nil)
        me.timer = BREAK_TICKS
        to_broken(me)
    elseif bit32.btest(c.hit_flags, BURSTING) then
        return burst(me)
    else
        local hp = me.hp
        local last = s.last_hp
        s.last_hp = hp
        if hp ~= last then
            me.timer = s.stop_ticks
            me:set_action(STOPPED)
        end
    end
    if not battle.dimmed() then
        local act = ACTIONS[me.action]
        if act == nil then
            error(string.format("rolling log action %#x reads past its table", me.action))
        end
        act(me, s)
    end
    c:present()
end

-- `sub_80D124E`: its look, HP and path.
local function init(me: Object, hp: number)
    local s = me.state :: State
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
    me:setup_collision(collision.own_object, collision.hit_by_other_side_or_blockers, HIT_MOD)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.visible = true
    me.anim = if s.big then 3 else 1
    local c = me.collision
    c:set_hit_effect(nil)
    me.hp = hp
    me.max_hp = hp
    s.last_hp = hp
    sprite.palette = 0
    c:set_region(nil)
    c:present()
    launch(me, s)
    me.lifecycle = "update"
end

log.kind = define.kind {
    id = "rlnglog/log",
    pool = "attack",
    state = { last_hp = "u16", big = "bool", stop_ticks = "u8", thrown = "bool" },
    -- `sub_80D1218`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- Its HP is its spawner's (`last_hp` holds it until here).
            init(me, (me.state :: State).last_hp)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        if me.active then
            me:update_sprite_while_paused()
        end
    end,
}

-- `sub_80D1442`: `what` for `owner` on (x, y), with the damage word
-- `damage`. It runs while dimmed. Its position is the spawner's registers
-- (the row, the element and a height of 16 pixels) until its init places
-- it; the height stays.
function log.spawn(owner: Object, x: number, y: number, element: number, what: RollingLog, damage: number): Object?
    local o = battle.spawn(log.kind, Vec3.new(y, element, HEIGHT * 0x10000))
    if o then
        o.element = element
        o.panel_x, o.panel_y = x, y
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = nil
        o.run_while_dimmed = true
        local s = o.state :: State
        s.big = what.big
        s.stop_ticks = what.stop_ticks
        s.thrown = what.thrown
        s.last_hp = what.hp
    end
    return o
end

return log
