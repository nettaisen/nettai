--!strict
-- EXE5's SpShakes (action 0x51, lib/armshot): an electric wave (attack object #0x68, 0x080D2FF8, EXE5's own) that
-- snakes along the field from the panel ahead, a row up and down as it goes (a sine of 24 pixels, one swing a
-- panel), hitting what it touches; after a hit it ends once it is on a body's panel again (unless it paralyzes:
-- then its hit comes back on the next panel). It goes off the field's columns.
--
-- Its parameters (the record's): Param1 the speed (1.5, 2.25, 3 pixels a tick: 0x080D30D4), Param2 set (hits
-- paralyze 90 ticks with no modifier: no chip has it), Param3 the palette. Its element is elec whatever the chip's
-- (the action passes 3).

local armshot = require("../../lib/armshot")
local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")
local status = require("../../rules/status")
local trajectory = require("../../lib/trajectory")

local spshake = {}

-- A chip's wave: its speed (16.16 a tick), whether it paralyzes, its palette.
export type Wave = { speed: number, paralyzes: boolean, palette: number }

export type State = {
    wave: Wave,
    -- The sine's step a tick (ExtraVars+8, 16.16 of a 256-step turn) and the row's center (ExtraVars+0xC).
    step: number,
    center_y: number,
    -- The panel it last hit on (ExtraVars+0: x, and y by 8).
    hit_at: number,
}

local SPRITE = asset.sprite("shake-wave")
local SOUND = asset.sound("wave")
local ELEC = 3
-- The sine: 24 pixels, one swing over 80 pixels (two panels) of travel; it starts at the middle of the turn.
local AMPLITUDE, SWING, START = 24, 0x50_0000, 0x8000
local BODIES = 0x0F800000

spshake.waves = {
    [1] = { speed = 0x1_8000, paralyzes = false, palette = 0 },
    [2] = { speed = 0x2_4000, paralyzes = false, palette = 6 },
    [3] = { speed = 0x3_0000, paralyzes = false, palette = 0x0C },
}

local function finish(me: Object)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- 0x080D315C: on along the row, swinging up and down.
local function move(me: Object, s: State)
    me.timer2 = int.u16(me.timer2 + s.step)
    local p = me.pos
    local x = int.i32(p.x + me.vel.x)
    local y = int.i32(trajectory.sine(bit32.rshift(me.timer2, 8)) * AMPLITUDE * 0x100 + s.center_y)
    me.pos = Vec3.new(x, y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    -- (0x0800B388: the panels of its region, so none while it has none: the tick after a hit.)
    me:highlight_collision_panels()
end

-- 0x080D30E0.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        return finish(me)
    end
    local s = me.state :: State
    local here = bit32.bor(me.panel_x, bit32.lshift(me.panel_y, 8))
    if c.region == nil then
        if s.wave.paralyzes then
            if here ~= s.hit_at then
                c:set_region(regions.single)
            end
        elseif not field.check(me.panel_x, me.panel_y, 0, BODIES) then
            me:set_lifecycle("destroy")
            return
        else
            c:set_region(regions.single)
        end
    end
    if c.hit_flags ~= 0 then
        c:set_region(nil)
        s.hit_at = here
    end
    local a = me.action
    if a ~= 0 then
        error(string.format("SpShake wave action %#x reads past its table", a))
    end
    move(me, s)
    if me.panel_x < 1 or me.panel_x > 6 then
        return finish(me)
    end
    c:present()
end

-- 0x080D301C.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    sprite.palette = s.wave.palette
    local vx = int.mul32(s.wave.speed, me.facing)
    me.vel = Vec3.new(vx, me.vel.y, me.vel.z)
    local per = int.tdiv(SWING, vx)
    s.step = int.tdiv(0x1_0000, math.abs(per))
    me.timer2 = START
    local _, cy = field.center(me.panel_x, me.panel_y)
    s.center_y = cy
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.thrown, collision.hits_navis, if s.wave.paralyzes then 0 else 3)
    if s.wave.paralyzes then
        me.collision.status_base = status.paralyze_90
    end
    me.collision:set_hit_effect(sparks.elec)
    me.collision:present()
    me:set_lifecycle("update")
    tick(me)
    battle.play_sound(SOUND)
end

spshake.kind = define.kind {
    id = "spshake/wave",
    pool = "attack",
    state = { wave = "record:shake-wave", step = "i32", center_y = "i32", hit_at = "u32" },
    -- 0x080D2FF8: by its lifecycle, then a sprite step.
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

for k, w in spshake.waves do
    spshake.waves[k] = define.record("shake-wave", w)
end

-- A wave by its parameters (ShakPar's: chips/shakpar/shakpar): Param1 the speed's row of 0x080D30D4, Param2 whether it
-- paralyzes, Param3 its palette.
local SPEEDS = { [0] = 0x1_8000, [1] = 0x2_4000, [2] = 0x3_0000 }
function spshake.wave(speed: number, paralyzes: boolean, palette: number): Wave
    local v = SPEEDS[speed]
    if not v then
        error(string.format("the SpShake wave's speed %d reads past 0x080D30D4", speed))
    end
    return define.record("shake-wave", { speed = v, paralyzes = paralyzes, palette = palette })
end

-- 0x080D319E: a wave of `wave` on (x, y) with `element` (the SpShakes' action passes elec) and the damage word.
function spshake.spawn(owner: Object, x: number, y: number, element: number, damage: number, wave: Wave): Object?
    local o = battle.spawn(spshake.kind, Vec3.new(y, element, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        local s = o.state :: State
        s.wave = wave
    end
    return o
end

-- A SpShake's action, firing `wave`.
function spshake.action(wave: Wave): Action
    return armshot.action {
        fire = function(me: Object, x: number, y: number, damage: number)
            spshake.spawn(me, x, y, ELEC, damage, wave)
        end,
    }
end

return spshake
