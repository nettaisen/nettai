--!strict
-- A bubble star (the original's attack object #0x42, `sub_80CC5A8`):
-- BblStar's. It drifts forward from its panel at its chip's speed,
-- swaying 24 pixels up and down the field along a sine wave that makes one
-- sway per 80 pixels travelled, and bubbles what it hits (status 0x60).
-- After a hit its region is off until it reaches another panel; a hit on
-- something that pops it (hit flag 0x800000) bursts it. It ends past the
-- field's side edges. A star on an invalid panel, or with no collision
-- slot, bursts at once.

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local regions = require("../../lib/regions")
local sparks = require("../../lib/sparks")
local statuses = require("../../rules/status")
local trajectory = require("../../lib/trajectory")

local star = {}

export type State = {
    -- Its speed (16.16 a tick) and palette, its chip's (the original's
    -- first parameter picks them from `byte_80CC65C`).
    speed: number,
    palette: number,
    -- The panel (x | y << 8) it last hit on (ExtraVars+0).
    hit_panel: number,
    -- The wave's phase step per tick (ExtraVars+8), and the row's center
    -- Y it sways around (ExtraVars+0xC).
    phase_step: number,
    center_y: number,
}

local SPRITE = asset.sprite("bubble-star")
local SOUND_HIT = asset.sound("bubble")
-- One full sway (a phase of 0x10000) per this far, 16.16.
local SWAY_LENGTH = 0x50_0000
-- The sway's height: 24 pixels at the sine's peak (256).
local SWAY = 0x18
-- What it does to what it hits: a bubble.
local STATUS = statuses.bubble_150
-- A hit on this bursts it.
local BURSTS = 0x80_0000

-- The puff it bursts in, 16 pixels above it.
local function puff(me: Object)
    local p = me.pos
    battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), effects.puff)
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
    local sine = trajectory.sine(bit32.rshift(phase, 8))
    me.pos = Vec3.new(x, int.i32(sine * SWAY * 0x100 + s.center_y), p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    local px = me.panel_x
    if px < 1 or px > 6 then
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
    end
    me:highlight_collision_panels()
end

-- `sub_80CC668`: resolve its hits; drift; its region comes back on a new
-- panel.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    local s = me.state :: State
    local hits = c.hit_flags
    if hits ~= 0 then
        if bit32.btest(hits, BURSTS) then
            puff(me)
            c:set_region(nil)
            me:set_lifecycle("destroy")
            return
        end
        s.hit_panel = panel_word(me)
        c:set_region(nil)
        battle.play_sound(SOUND_HIT)
    end
    local action = me.action
    if action ~= 0 then
        error(string.format("bubble star action %#x reads past off_80CC6E4", action))
    end
    drift(me)
    local highlight = true
    if c.region == nil then
        if panel_word(me) == s.hit_panel then
            highlight = false
        else
            c:set_region(regions.single)
        end
    end
    if highlight then
        field.highlight(me.panel_x, me.panel_y)
    end
    me:update_sprite()
    c:present()
end

-- `sub_8011504`: the sprite with a ground shadow, then collision (false
-- when no slot was free).
local function setup(me: Object): boolean
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite:set_animation(0)
    sprite.shadow = "ground"
    me.anim = 0
    me.anim_loaded = 0xFF
    if not me:create_collision() then
        return false
    end
    me:setup_collision(collision.attack, collision.hits_navis, 0)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    return true
end

-- `sub_80CC5C8`: on its panel, its speed and a wave.
local function init(me: Object)
    me:set_coordinates_from_panel()
    if not field.valid(me.panel_x, me.panel_y) or not setup(me) then
        puff(me)
        me:free()
        return
    end
    me.visible = true
    local s = me.state :: State
    me.sprite.palette = s.palette
    local vel = me.facing * s.speed
    me.vel = Vec3.new(vel, me.vel.y, me.vel.z)
    -- Ticks to cross one sway, then the phase step (BIOS Div, truncating).
    local ticks = math.abs(int.tdiv(SWAY_LENGTH, vel))
    if ticks == 0 then
        error("bubble star: its sway divides by zero (sub_80CC5C8)")
    end
    s.phase_step = int.tdiv(0x1_0000, ticks)
    me.timer2 = 0x8000
    local _, cy = field.center(me.panel_x, me.panel_y)
    s.center_y = cy
    local c = me.collision
    c:set_hit_effect(sparks.aqua)
    c.status_base = STATUS
    c:present()
    me.lifecycle = "update"
    tick(me)
end

star.kind = define.kind {
    id = "bblstar/star",
    pool = "attack",
    state = { speed = "i32", palette = "u8", hit_panel = "u32", phase_step = "i32", center_y = "i32" },
    -- `sub_80CC5A8`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
    end,
}

-- `sub_80CC73C`: a star for `owner` on (x, y) with the damage word
-- `damage`, at `speed` (16.16 a tick) in `palette`. Its position is the
-- spawner's registers (the panel Y, the element and 0) until its init
-- places it.
function star.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    damage: number,
    speed: number,
    palette: number
): Object?
    local o = battle.spawn(star.kind, Vec3.new(y, element, 0))
    if o then
        local s = o.state :: State
        s.speed, s.palette = speed, palette
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

return star
