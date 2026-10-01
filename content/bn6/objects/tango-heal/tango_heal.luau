--!strict
-- Tango's heal (attack object #0xC7, `sub_80DE000`): thrown by Tango
-- (objects/tango), it arcs onto her navi's panel in 20 ticks; landed, it
-- tells her (her `healing` flag), leaves a sparkle, heals her navi 300 HP
-- and gives it barrier 5 (Barr100's, lib/barriers) with the barrier's
-- visual. See
-- docs/engine/chips.md §2.10.

local barriers = require("../../lib/barriers/barriers")
local trajectory = require("../../lib/trajectory")

local tango_heal = {}

local SPRITE = "0C-4D"
local FLIGHT_TICKS = 0x14
-- The arc's gravity (Z velocity, 16.16 a tick: 0xFFFF7778).
local GRAVITY = -0x8888
local HEAL = 300
local HEAL_SOUND = 0x8A
-- Effect #0's look as it lands.
local SPARKLE = 6

-- `sub_80DE112`: the heal from `tango` at `pos`, for her navi's panel
-- (x, y) (its third and fourth parameters).
function tango_heal.spawn(tango: Object, pos: Vec3, x: number, y: number): Object?
    local o = battle.spawn_kind("tango-heal", pos, { 0, 0, x, y })
    if not o then
        return nil
    end
    o.alliance = tango.alliance
    o.related1 = tango
    local s = tango.state :: { healing: number }
    s.healing = 1
    o.run_while_dimmed = true
    return o
end

-- `sub_80DE024`: its look, and the arc onto the panel.
local function init(me: Object)
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite:step()
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    local x, y = field.center(me:param(3), me:param(4))
    me.vel = trajectory.in_ticks(me.pos, x, y, 0, FLIGHT_TICKS, GRAVITY)
    me.timer = FLIGHT_TICKS
    me.lifecycle = "update"
end

-- `sub_80DE088`: the arc; landed, the heal.
local function tick(me: Object)
    if bit32.band(me.sprite.frame_flags, 0x80) ~= 0 then
        me.anim, me.anim_loaded = 1, 0
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t >= 0 then
        local p, v = me.pos, me.vel
        me.pos = Vec3.new(int.i32(p.x + v.x), int.i32(p.y + v.y), int.i32(p.z + v.z))
        me.vel = Vec3.new(v.x, v.y, int.i32(v.z + GRAVITY))
        me:set_panel_from_coordinates()
        return
    end
    local tango = me.related1 :: Object
    local s = tango.state :: { healing: number }
    s.healing = 0
    me:set_lifecycle("destroy")
    battle.effect(me.pos, SPARKLE)
    local navi = battle.player(me.alliance)
    if not navi then
        -- The game heals and shields through a null pointer.
        error("Tango's heal has no navi to heal (sub_80DE088 reads a null object)")
    end
    navi.hp = math.min(navi.hp + HEAL, navi.max_hp)
    battle.play_sound(HEAL_SOUND)
    -- sub_801A7CC(5), then the barrier's visual: the navi's old one (its
    -- AIData+0x60) goes (sub_80E0DC0) and a new one comes (effect object
    -- #7, sub_80E0D98).
    barriers.raise(navi, barriers.barrier_100)
end

-- `sub_80DE000`: by state, then its sprite.
function tango_heal.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
    me:update_sprite()
end

return tango_heal
