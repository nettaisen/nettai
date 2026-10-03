--!strict
-- BN5's Pulsars (action 0x52, lib/armshot): a pulse (attack object #0x6A, 0x080D32D0, BN5's own; BN6 numbers its
-- tomahawk strike so) flies along the row 24 pixels up, 8 pixels a tick, hitting what it touches; on a body it
-- slows to a quarter and 8 ticks later bursts over the eight panels around where it hit, paralyzing for 90 ticks
-- (a hit that lasts 26 ticks). Anything else it touches ends it; so does leaving the field.
--
-- Its parameters (the record's, the same for the three chips): Param1 the burst's hit modifier, Param2 its status,
-- Param3 set (no panel highlight), Param4 set (its touch hits with modifier 1 and no status).

local armshot = require("../../lib/armshot")
local collision = require("../../rules/collision")
local regions = require("@common/regions")
local status = require("../../rules/status")

local pulsar = {}

export type State = {
    -- The panel it hit (ExtraVars+0: x, and y by 4).
    hit_at: number,
}

local SPRITE = asset.sprite("bn5:pulsar")
local SOUND, BURST_SOUND = asset.sound("bn5:pulsar"), asset.sound("bn5:elec-pulse")
local BURST = define.effect { sprite = asset.sprite("bn5:pulsar-burst"), anim = 0, palette = 0 }
local HEIGHT, SPEED = 0x18_0000, 0x8_0000
-- What it may touch and go on bursting: bodies.
local BODIES = 0x03800000
-- The burst: 8 ticks after the hit, over the ring around it; its hit lasts 26 ticks.
local BURST_DELAY, BURST_TICKS = 8, 26
-- The chips' parameters: the burst's hit modifier and status; the touch's hit modifier (Param4 set).
local BURST_HIT_MOD, BURST_STATUS, TOUCH_HIT_MOD = 0, status.paralyze_90, 1
local FLY, BURST_ACTION = 0x0, 0x4

local function finish(me: Object)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- 0x080D33E8: fly on.
local function fly(me: Object)
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x + me.vel.x), p.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    -- (Param3 set: no highlight.)
end

-- 0x080D3406: slow down, then burst around the panel hit.
local function burst(me: Object, s: State)
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x + me.vel.x), p.y, p.z)
    local t = me.timer - 1
    me.timer = t
    if t >= 0 then
        return
    end
    local cx, cy = field.center(me.panel_x, me.panel_y)
    battle.effect(Vec3.new(cx, cy, 0x8_0000), BURST, bit32.bxor(me.alliance, me.flip))
    local box = battle.hitbox(me, {
        panel_x = bit32.band(s.hit_at, 7),
        panel_y = bit32.rshift(s.hit_at, 4),
        element = me.element,
        z = 0,
        region = regions.ring,
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.attack,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = BURST_HIT_MOD,
        status = BURST_STATUS,
    })
    if box then
        box.timer = BURST_TICKS
        battle.play_sound(BURST_SOUND)
    end
    me:set_lifecycle("destroy")
end

-- 0x080D337A: resolve its hits: a body slows it to burst, anything else ends it; by its action; it goes off the
-- field.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        return finish(me)
    end
    local hit = c.hit_flags
    if hit ~= 0 then
        if not bit32.btest(hit, BODIES) then
            return finish(me)
        end
        c:set_region(nil)
        local s = me.state :: State
        s.hit_at = bit32.bor(me.panel_x, bit32.lshift(me.panel_y, 4))
        me.timer = BURST_DELAY
        me.vel = Vec3.new(int.asr(me.vel.x, 2), me.vel.y, me.vel.z)
        me.action = BURST_ACTION
        me.phase, me.phase_init = 0, 0
    end
    local a = me.action
    if a == FLY then
        fly(me)
    elseif a == BURST_ACTION then
        burst(me, me.state :: State)
    else
        error(string.format("Pulsar action %#x reads past its table", a))
    end
    if not field.valid(me.panel_x, me.panel_y) then
        return finish(me)
    end
    c:present()
end

-- 0x080D32F4.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.vel = Vec3.new(int.mul32(me.facing, SPEED), me.vel.y, me.vel.z)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.attack, collision.hits_navis, TOUCH_HIT_MOD)
    me.collision:set_hit_effect(nil)
    me.collision:present()
    me:set_lifecycle("update")
    tick(me)
    battle.play_sound(SOUND)
end

pulsar.kind = define.kind {
    id = "bn5:pulsar/pulse",
    pool = "attack",
    state = { hit_at = "u32" },
    -- 0x080D32D0: by its lifecycle, then a sprite step.
    update = function(me: Object)
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
    end,
}

-- 0x080D3464: a pulse on (x, y), 24 pixels up, with `element` and the damage word.
local function spawn(owner: Object, x: number, y: number, element: number, damage: number): Object?
    local o = battle.spawn(pulsar.kind, Vec3.new(y, element, HEIGHT))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.vel = Vec3.new(SPEED, 0, 0)
    end
    return o
end

-- The Pulsars' action.
pulsar.action = armshot.action {
    id = "bn5:pulsar/action",
    fire = function(me: Object, x: number, y: number, damage: number)
        spawn(me, x, y, me.attack_element, damage)
    end,
}

return pulsar
