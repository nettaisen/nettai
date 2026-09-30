--!strict
-- SunMoon, the navi of the SunMoon Program Advance (actor object #0x24,
-- `sub_80BF260`), brought by its navi chip controller: a sun-and-moon
-- above its user's panel. It throws six fire meteors at the panel three
-- columns ahead, 15 ticks apart (objects/sun-meteor); flashes; turns to
-- the moon and shines on that panel for 100 ticks (objects/moon-beam);
-- flashes back; and dives onto it, hitting its panel and those around it
-- (region 0xF), cracking them, with a palette flash. Then it goes and the
-- controller moves on. See docs/engine/chips.md §3.6.32.
--
-- The chip's script (navi chip subtype 25): `navi_chip` is its spawner,
-- `off_802CD5C[25]`.

local panels = require("../../lib/panels")
local sun_meteor = require("../sun-meteor/sun_meteor")
local moon_beam = require("../moon-beam/moon_beam")

export type State = {
    -- The controller waiting for it (ExtraVars[0]).
    controller: Object?,
}

local sun_moon = {
    state = { controller = "object" },
}

local SPRITE = "0C-64"

-- Where it hovers (16.16).
local HEIGHT = 0x40_0000

-- The dive: 16 ticks from its height to the panel three columns toward
-- the enemy.
local DIVE_TICKS = 0x10

-- What its landing panel must have one of for the dive to hit (solid or
-- occupied: as one word).
local HOLDS = 0x0F80_0010

-- The region its dive hits, cracks and marks.
local DIVE_REGION = 0x0F

-- `sub_80BF6AE`: SunMoon for `user`, reporting to `controller`. It runs
-- while dimmed. (Its position is the spawner's registers, which its init
-- overwrites.)
function sun_moon.navi_chip(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local o = battle.spawn_kind("sun-moon", Vec3.zero, spec.params)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = spec.panel_x, spec.panel_y
    o.element = spec.element
    o.damage = spec.damage
    o.stamina = bit32.rshift(spec.damage, 16)
    o.related1 = user
    local s = o.state :: State
    s.controller = controller
    o.alliance, o.flip = user.alliance, user.flip
    o.run_while_dimmed = true
    return o
end

-- The damage word's flag bits (0xF000 of the damage) with `damage`.
local function flagged(me: Object, damage: number): number
    return bit32.bor(damage, bit32.band(me.damage, 0xF000))
end

-- Enter the phase with a timer, or count it down; true once it is 0 or
-- below.
local function countdown(me: Object, ticks: number): boolean
    if me.phase_init == 0 then
        me.timer = ticks
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = left
    return left <= 0
end

-- `sub_80BF2FC`: it appears; 60 ticks.
local function appear(me: Object)
    if me.phase_init == 0 then
        battle.play_sound(0x94)
    end
    if countdown(me, 0x3C) then
        me:set_action(4)
    end
end

-- `sub_80BF340`: a meteor every 15 ticks, six of them.
local function meteors(me: Object)
    if me.phase_init == 0 then
        me.anim = 2
        me.timer = 0
        me.timer2 = 6
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    me.timer = 0xF
    sun_meteor.spawn(me, me.panel_x, me.panel_y, 1, { 0 }, flagged(me, 0x32), me.pos.z)
    left = me.timer2 - 1
    me.timer2 = left
    if left > 0 then
        return
    end
    me.phase, me.phase_init = 4, 0
end

-- `sub_80BF38A`, `sub_80BF428`, `sub_80BF548`: 30 ticks (the first two
-- in animation 1), then `action`.
local function hold(action: number, anim: number?): (Object) -> ()
    return function(me: Object)
        if me.phase_init == 0 and anim then
            me.anim = anim
        end
        if countdown(me, 0x1E) then
            me:set_action(action)
        end
    end
end

-- `sub_80BF402` (`sub_80BF522` inverted): its palette blinks, every
-- other tick above 20 ticks left, every fourth below (the routine's third
-- rate is never reached).
local function blink(me: Object, timer: number, inverted: boolean)
    local rate = if timer > 0x14 then 2 else 4
    local lit = bit32.band(timer, rate) ~= 0
    if inverted then
        lit = not lit
    end
    me.sprite.palette = if lit then 4 else 0
end

-- `sub_80BF3CC` / `sub_80BF4EC`: 30 ticks of blinking (sound 0x110),
-- ending on the moon's palette (4) or the sun's (0).
local function turn(inverted: boolean): (Object) -> ()
    return function(me: Object)
        if me.phase_init == 0 then
            me.anim = 1
            me.timer = 0x1E
            battle.play_sound(0x110)
            me.phase_init = 4
        end
        blink(me, me.timer, inverted)
        local left = me.timer - 1
        me.timer = left
        if left > 0 then
            return
        end
        me.sprite.palette = if inverted then 0 else 4
        me.phase, me.phase_init = 4, 0
    end
end

-- `sub_80BF464`: the moonlight; 100 ticks.
local function moonlight(me: Object)
    if me.phase_init == 0 then
        me.timer = 0x64
        me.anim = 3
        me.sprite.palette = 0
        moon_beam.spawn(me, me.panel_x, me.panel_y, 0, { 0 }, flagged(me, 2))
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me.phase, me.phase_init = 4, 0
    end
end

-- `sub_80BF4A4`: 30 ticks as the moon.
local function after_moonlight(me: Object)
    if me.phase_init == 0 then
        me.timer = 0x1E
        me.anim = 1
        me.sprite.palette = 4
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_action(0x10)
    end
end

-- `sub_80BF588`: 10 ticks (the camera shakes for them, magnitude 1); then
-- aim the dive at the panel three columns toward the enemy (sound 0x17F).
local function aim_dive(me: Object)
    if me.phase_init == 0 then
        battle.shake_camera(1, 0xA)
    end
    if countdown(me, 0xA) then
        me.timer = DIVE_TICKS
        local direction = 1 - 2 * me.alliance
        local target = field.center(me.panel_x + direction * 3, me.panel_y)
        local p = me.pos
        me.vel = Vec3.new(int.tdiv(int.i32(target - p.x), DIVE_TICKS), 0, int.tdiv(int.i32(-p.z), DIVE_TICKS))
        battle.play_sound(0x17F)
        me.phase, me.phase_init = 4, 0
    end
end

-- `sub_80BF5E0`: dive; landing on a panel that holds something, the hit
-- (region 0xF), its panels cracked and marked, a palette flash; on one
-- that doesn't, just a burst. Then it is out of sight.
local function dive(me: Object)
    local left = me.timer - 1
    me.timer = left
    if left ~= 0 then
        local p, v = me.pos, me.vel
        me.pos = Vec3.new(p.x + v.x, p.y, p.z + v.z)
        me:set_panel_from_coordinates()
        me:update_collision_panels()
        return
    end
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    local x, y = me.panel_x, me.panel_y
    if bit32.band(field.flags(x, y), HOLDS) == 0 then
        battle.effect(Vec3.new(p.x, p.y, 0x10_0000), 0x12)
    else
        battle.play_sound(0xC3)
        battle.shake_camera(3, 0x1E)
        local box = battle.hitbox(me, {
            panel_x = x,
            panel_y = y,
            element = 0,
            z = 0,
            region = DIVE_REGION,
            hit_effect = 0xFF,
            target = 5,
            self_type = 0x0A,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = 3,
        })
        if box then
            box.run_while_dimmed = true
        end
        panels.crack_region(x, y, DIVE_REGION)
        -- sub_801BD3C, with the 3 left in r7 as the marks' Z.
        battle.region_effects(x, y, DIVE_REGION, me.alliance, 5, 3)
        battle.palette_flash(0, 0x1E, true)
    end
    me.phase, me.phase_init = 8, 0
    me.visible = false
end

-- `sub_80BF67E`: 60 ticks, then it goes.
local function linger(me: Object)
    if countdown(me, 0x3C) then
        me:set_lifecycle("destroy")
    end
end

local function by_phase(name: string, table_name: string, phases: { [number]: (Object) -> () }): (Object) -> ()
    return function(me: Object)
        local f = phases[me.phase]
        if not f then
            error(string.format("SunMoon's %s phase %#x reads past its table (%s)", name, me.phase, table_name))
        end
        f(me)
    end
end

local ACTIONS = {
    [0] = appear,
    [4] = by_phase("meteors", "off_80BF338", { [0] = meteors, [4] = hold(8, 1) }),
    [8] = by_phase("turning", "off_80BF3C4", { [0] = turn(false), [4] = hold(0xC) }),
    [0xC] = by_phase("moonlight", "off_80BF45C", { [0] = moonlight, [4] = after_moonlight }),
    [0x10] = by_phase("turning back", "off_80BF4E4", { [0] = turn(true), [4] = hold(0x14) }),
    [0x14] = by_phase("dive", "off_80BF57C", { [0] = aim_dive, [4] = dive, [8] = linger }),
}

-- `sub_80BF2D0`.
local function tick(me: Object)
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("SunMoon action %#x reads past its table (off_80BF2E4)", me.action))
    end
    action(me)
end

-- `sub_80BF284`: above its panel; then its first tick.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim = 0
    me.anim_loaded = 0xFF
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, HEIGHT)
    me.visible = true
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80BF69C`: the controller moves on.
local function leave(me: Object)
    local controller = (me.state :: State).controller
    if controller then
        navi_chip.navi_left(controller)
    end
    me:free()
end

-- `sub_80BF260`.
function sun_moon.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        leave(me)
    end
    me:update_sprite()
end

return sun_moon
