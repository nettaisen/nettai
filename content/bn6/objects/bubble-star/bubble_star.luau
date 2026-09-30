--!strict
-- A bubble star (attack object #0x42, `sub_80CC5A8`): BblStar's. It drifts
-- forward from its panel at the speed its first parameter picks, swaying
-- 24 pixels up and down the field along a sine wave that makes one sway
-- per 80 pixels travelled, and bubbles what it hits (status 0x60). After a
-- hit its region is off until it reaches another panel; a hit on something
-- that pops it (hit flag 0x800000) bursts it. It ends past the field's
-- side edges. A star on an invalid panel, or with no collision slot,
-- bursts at once (effect 0x14).

local object_setup = require("../../lib/object_setup")

export type State = {
    -- The panel (x | y << 8) it last hit on (ExtraVars+0).
    hit_panel: number,
    -- The wave's phase step per tick (ExtraVars+8), and the row's center
    -- Y it sways around (ExtraVars+0xC).
    phase_step: number,
    center_y: number,
}

local bubble_star = {
    state = { hit_panel = "u32", phase_step = "i32", center_y = "i32" },
}

local SPRITE = "04-0f"
-- Speed by the first parameter (`byte_80CC65C`), 16.16 per tick.
local SPEEDS = { [0] = 0x1_8000, 0x2_4000, 0x3_0000 }
-- One full sway (a phase of 0x10000) per this far, 16.16.
local SWAY_LENGTH = 0x50_0000
-- The sway's height: 24 pixels at the sine's peak (256).
local SWAY = 0x18
-- What it does to what it hits.
local HIT_EFFECT, STATUS = 2, 0x60
-- A hit on this bursts it.
local BURSTS = 0x80_0000

-- `sub_80CC73C`: a star for `owner` on (x, y), with the damage word
-- `damage` and the chip's `params`. Its position is the spawner's
-- registers (the panel Y, the element and 0) until its init places it.
function bubble_star.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    damage: number,
    params: { number }
): Object?
    local o = battle.spawn_kind("bubble-star", Vec3.new(y, element, 0), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

-- The object's panel as the game's halfword (x | y << 8).
local function panel_word(me: Object): number
    return me.panel_x + me.panel_y * 0x100
end

-- `sub_80CC6E8`: forward along the wave; gone past the field's sides.
local function drift(me: Object)
    local s = me.state :: State
    local phase = int.u16(me.timer2 + s.phase_step)
    me.timer2 = phase
    local p = me.pos
    local x = int.i32(p.x + me.vel.x)
    local sine = data.rules.sine[bit32.rshift(phase, 8)]
    me.pos = Vec3.new(x, int.i32(sine * SWAY * 0x100 + s.center_y), p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    local px = me.panel_x
    if px < 1 or px > 6 then
        me.collision.region = 0
        me:set_lifecycle("destroy")
    end
    object_setup.highlight_collision_panels(me)
end

-- `sub_80CC668`: resolve its hits; drift; its region comes back on a new
-- panel.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    local hits = c.hit_flags
    if hits ~= 0 then
        if bit32.btest(hits, BURSTS) then
            object_setup.puff(me)
            c.region = 0
            me:set_lifecycle("destroy")
            return
        end
        (me.state :: State).hit_panel = panel_word(me)
        c.region = 0
        battle.play_sound(0x12D)
    end
    local action = me.action
    if action ~= 0 then
        error(string.format("bubble star action %#x reads past off_80CC6E4", action))
    end
    drift(me)
    local highlight = true
    if c.region == 0 then
        if panel_word(me) == (me.state :: State).hit_panel then
            highlight = false
        else
            c.region = 1
        end
    end
    if highlight then
        field.highlight(me.panel_x, me.panel_y)
    end
    me:update_sprite()
    c:present()
end

-- `sub_80CC5C8`: on its panel, a speed and a wave by its first parameter.
local function init(me: Object)
    me:set_coordinates_from_panel()
    if
        not field.valid(me.panel_x, me.panel_y)
        or not object_setup.sprite_and_collision(me, SPRITE, 0, true, 0x04, 0x05, 0x00)
    then
        object_setup.puff(me)
        me:free()
        return
    end
    me.visible = true
    local kind = me:param(1)
    me.sprite.palette = kind
    local speed = SPEEDS[kind]
    if speed == nil then
        error(string.format("bubble star kind %d reads past byte_80CC65C", kind))
    end
    local vel = me.facing * speed
    me.vel = Vec3.new(vel, me.vel.y, me.vel.z)
    -- Ticks to cross one sway, then the phase step (BIOS Div, truncating).
    local ticks = math.abs(int.tdiv(SWAY_LENGTH, vel))
    if ticks == 0 then
        error("bubble star: its sway divides by zero (sub_80CC5C8)")
    end
    local s = me.state :: State
    s.phase_step = int.tdiv(0x1_0000, ticks)
    me.timer2 = 0x8000
    local _, cy = field.center(me.panel_x, me.panel_y)
    s.center_y = cy
    local c = me.collision
    c.hit_effect = HIT_EFFECT
    c.status_base = STATUS
    c:present()
    me.lifecycle = "update"
    tick(me)
end

-- `sub_80CC5A8`.
function bubble_star.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
end

return bubble_star
