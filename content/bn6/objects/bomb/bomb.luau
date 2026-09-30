--!strict
-- A thrown bomb (attack object #8, `sub_80C5BB0`): MiniBomb, EnergBom,
-- MegEnBom and BigBomb (the bomb and seed chips' action 0x12). It flies
-- three panels ahead in an arc for 40 ticks, then lands: on a solid panel
-- it blasts its region (with an explosion on each panel) and a tick later
-- ends, leaving its after-blast for kinds 1 and 2; off the field or on a
-- hole it just ends (a splash in the hole). What it is, its first
-- parameter:
--
--   0  MiniBomb: blasts its own panel.
--   1  (no chip): no blast; leaves a lingering hit (objects/bomb-slash).
--   2  EnergBom, MegEnBom: no blast; leaves the energy burst
--      (objects/energy-burst), three hits on the panel.
--   3  BigBomb: blasts the 3x3 around it.
--
-- Until it lands its panel is (0, 0), off the field, so what it presents
-- hits nothing.

local bomb_slash = require("../bomb-slash/bomb_slash")
local energy_burst = require("../energy-burst/energy_burst")
local region = require("../../lib/region")

local bomb = {}

local SPRITE = "0c-02"
-- Flight: 40 ticks at 0x2E666 a tick forward, rising at 0x20666 less
-- 0x2800 a tick (`byte_80C5D58`; the gravity sits in the Y velocity).
local FLIGHT_TICKS = 0x28
local SPEED, RISE, GRAVITY = 0x2E666, 0x20666, -0x2800
-- The after-blast's lingering hit lasts 60 ticks.
local SLASH_TICKS = 0x3C
local SOUND_HIT_BOMB = 0x70
-- The explosion effect (effect object #0's), the hole splash's, and the
-- one the bomb leaves when the battle ends under it.
local EFFECT_SPLASH, EFFECT_GONE = 1, 0x14

-- By kind (`byte_80C5BA0`): collision self type, target type, hit
-- modifier, palette; then the blast's region (`dword_80C5D7C`) and its
-- effect (`dword_80C5D80`).
type Kind = { self_type: number, target: number, hit_mod: number, palette: number, region: number, effect: number }
local KINDS: { [number]: Kind } = {
    [0] = { self_type = 0x0A, target = 0x05, hit_mod = 0, palette = 0, region = 0x01, effect = 0 },
    [1] = { self_type = 0x04, target = 0x05, hit_mod = 3, palette = 1, region = 0x00, effect = 0 },
    [2] = { self_type = 0x0A, target = 0x05, hit_mod = 1, palette = 0, region = 0x00, effect = 0 },
    [3] = { self_type = 0x0A, target = 0x05, hit_mod = 3, palette = 3, region = 0x0F, effect = 0 },
}

local function kind_of(me: Object): Kind
    local k = KINDS[me:param(1)]
    if not k then
        error(string.format("bomb kind %d reads past byte_80C5BA0", me:param(1)))
    end
    return k
end

-- `sub_80C5DBC`: a bomb thrown by `user` from `pos`, with the chip's
-- parameters and damage word. It is null-element.
function bomb.throw(user: Object, pos: Vec3, params: { number }, damage: number): Object?
    local o = battle.spawn_kind("bomb", pos, params)
    if o then
        o.element = 0
        o.alliance, o.flip = user.alliance, user.flip
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.related1 = user
    end
    return o
end

-- Done: hidden, off its panels, and freed at the next update.
local function finish(me: Object)
    me.visible = false
    me.collision.region = 0
    me:set_lifecycle("destroy")
end

-- `sub_80C5C9C`: fly; on the last tick, land.
local function fly(me: Object)
    if me.phase_init == 0 then
        me.vel = Vec3.new(SPEED * me.facing, GRAVITY, RISE)
        me.timer = FLIGHT_TICKS
        me.phase_init = 4
    end
    local p, v = me.pos, me.vel
    local vz = int.i32(v.z + v.y)
    me.pos = Vec3.new(p.x + v.x, p.y, p.z + vz)
    me.vel = Vec3.new(v.x, v.y, vz)
    local t = me.timer - 1
    me.timer = t
    if t ~= 0 then
        return
    end
    me.visible = false
    me:set_panel_from_coordinates()
    local x, y = me.panel_x, me.panel_y
    me.collision.panel_x, me.collision.panel_y = x, y
    if not field.valid(x, y) then
        finish(me)
        return
    end
    if not field.solid(x, y) then
        battle.effect(me.pos, EFFECT_SPLASH)
        finish(me)
        return
    end
    local k = kind_of(me)
    me.collision.region = k.region
    region.effects(x, y, k.effect, k.region, region.direction(me.alliance), 0)
    battle.play_sound(SOUND_HIT_BOMB)
    me.action = 4
    me.phase, me.phase_init = 0, 0
end

-- `sub_80C5D84`: the tick after landing: leave the after-blast, and end.
local function landed(me: Object)
    me.collision.region = 0
    local kind = me:param(1)
    if kind == 1 then
        bomb_slash.spawn(me, me.panel_x, me.panel_y, me.element, SLASH_TICKS, me.damage, me.stamina)
    elseif kind == 2 then
        energy_burst.spawn(me, me.panel_x, me.panel_y, me.element, me.damage, me.stamina)
    end
    me:set_lifecycle("destroy")
end

-- `sub_80C5C40`: resolve last tick's hits; end with the battle or on a
-- hit; stand still while dimmed.
local function tick(me: Object)
    me.collision:remove()
    me:hit_spark()
    if battle.over() then
        battle.effect(me.pos, EFFECT_GONE)
        finish(me)
        return
    end
    if not battle.dimmed() then
        if me.collision.hit_flags ~= 0 then
            finish(me)
            return
        end
        local a = me.action
        if a == 0 then
            fly(me)
        elseif a == 4 then
            landed(me)
        else
            error(string.format("bomb action %#x reads past its table", a))
        end
    end
    me.collision:present()
end

-- `sub_80C5BD4`.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim = if me:param(1) == 2 then 3 else 1
    me.anim_loaded = 0xFF
    local k = kind_of(me)
    sprite.palette = k.palette
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(k.self_type, k.target, k.hit_mod)
    me.collision.hit_effect = 0xFF
    me.collision:present()
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80C5BB0`.
function bomb.update(me: Object)
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
end

return bomb
