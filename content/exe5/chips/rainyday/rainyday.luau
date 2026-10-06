--!strict
-- EXE5's RainyDay (the dimming handler's subtype 61): EXE5's own. Its controller (effect object #0x75, 0x080E908C: the
-- usual dimming phases; its effect, 0x080E90D0) finds the first enemy navi ahead (by columns from the panel ahead,
-- rows 1 to 3) and puts a rain cloud over it (attack object #0x97, 0x080D97A8), then waits for the cloud. With no
-- navi ahead it is done at once.
--
-- The cloud, 60 pixels up, waits 27 ticks, then rains (animation 1, with the rain under it: effect row 0x61): every
-- 10 ticks it looks over the field, from the far column toward its side's, rows 1 to 3 (going on where it left off),
-- for a sea panel of its own side; for each, its panel takes a hit (aqua, 16 pixels up, hit modifier 3) and the sea
-- panel turns normal with a splash (effect row 0x29). When none is left it stops raining, and 20 ticks later fades
-- (animation 2) for 25 and lets the controller go.

local collision = require("../../rules/collision")
local dimming_chips = require("@exelib/dimming")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local rainyday = {}

-- ---- The cloud ------------------------------------------------------------------------------------------------------

-- Its look (`sub_801BE2A`'s word 0x01000C36: sheet 0c-36, animation 0, a ground shadow), 60 pixels up.
local SPRITE = asset.sprite("rain-cloud")
local Z = 0x3C_0000
-- The rain under it (effect row 0x61: 0c-37, 20 pixels lower) and a turned panel's splash (row 0x29: 0c-13 animation 1).
local RAIN = new.effect { sprite = asset.sprite("rain"), anim = 0, palette = 0 }
local SPLASH = new.effect { sprite = asset.sprite("grab-shot"), anim = 1, palette = 0 }
local SOUND = asset.sound("drip-shower")
-- Its hit (`sub_80C53A6`'s word 0x04050201: its panel, the aqua spark, an attack that hits navis; hit modifier 3),
-- 16 pixels up.
local HIT_MOD, HIT_Z = 3, 0x10_0000
local SEA = "sea"
-- Its actions.
local WAIT, RAINING, DONE, FADE = 0, 4, 8, 0xC

export type CloudState = {
    -- Where its look over the field goes on (+0x60, +0x64) and which way (+0x68: toward its own side).
    scan_x: number,
    scan_y: number,
    dir: number,
    -- The rain under it (+0x6C).
    rain: Object?,
    -- The controller it lets go (+0x70: the address of the controller's flag).
    controller: Object?,
}

-- 0x080D9966: a hit on its panel.
local function hit(me: Object)
    local o = battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        z = HIT_Z,
        region = regions.single,
        hit_effect = sparks.aqua,
        target = collision.hits_navis,
        self_type = collision.attack,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = HIT_MOD,
    })
    if o then
        o.run_while_dimmed = true
    end
end

-- 0x080D997E: from where it left off, the next of its side's sea panels (by rows, then columns toward its side), and
-- where to go on from; none once off the field.
local function next_sea(me: Object, s: CloudState): (number?, number?)
    local x, y = s.scan_x, s.scan_y
    while field.valid(x, y) do
        local p = field.panel(x, y)
        if p and p.alliance == me.alliance and p.kind == SEA then
            if y + 1 > 3 then
                s.scan_x, s.scan_y = x + s.dir, 1
            else
                s.scan_x, s.scan_y = x, y + 1
            end
            return x, y
        end
        y += 1
        if y > 3 then
            y = 1
            x += s.dir
        end
    end
    return nil, nil
end

-- Action 0 (0x080D9814): 27 ticks.
local function wait(me: Object)
    if me.phase_init == 0 then
        me.timer = 27
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t <= 0 then
        me.timer2 = 0
        me:set_action(RAINING)
    end
end

-- Action 4 (0x080D983A): the rain's sound every 16 ticks; at the start the rain and its first hit; every 10 ticks a
-- sea panel taken and a hit; none left, done. Meanwhile the rain lasts.
local function raining(me: Object)
    local s = me.state :: CloudState
    local t2 = me.timer2 - 1
    me.timer2 = int.u16(t2)
    if t2 <= 0 then
        battle.play_sound(SOUND)
        me.timer2 = 16
    end
    if me.phase_init == 0 then
        me.anim = 1
        me.timer = 10
        s.dir = -me.facing
        s.scan_x = bit32.bxor(bit32.bxor(me.alliance, me.flip), 1) * 5 + 1
        s.scan_y = 1
        hit(me)
        local p = me.pos
        local rain = battle.effect(p, RAIN)
        s.rain = rain
        if rain then
            local q = rain.pos
            rain.pos = Vec3.new(q.x, q.y, int.i32(q.z - 0x14_0000))
        end
        me.pos = Vec3.new(p.x, int.i32(p.y + 0x1_0000), int.i32(p.z + 0x1_0000))
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t > 0 then
        if s.rain then
            (s.rain :: Object).timer = 3
        end
        return
    end
    me.timer = 10
    local x, y = next_sea(me, s)
    if x == nil then
        me:set_action(DONE)
        return
    end
    hit(me)
    field.set_type(x, y :: number, "normal")
    local cx, cy = field.center(x, y :: number)
    battle.effect(Vec3.new(cx, cy, 0), SPLASH)
end

-- Action 8 (0x080D98F2): 20 ticks.
local function done(me: Object)
    if me.phase_init == 0 then
        me.timer = 20
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t <= 0 then
        me:set_action(FADE)
    end
end

-- Action 0xC (0x080D9914): fading 25 ticks; the controller let go; gone.
local function fade(me: Object)
    if me.phase_init == 0 then
        me.anim = 2
        me.timer = 25
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t <= 0 then
        local c = (me.state :: CloudState).controller
        if c then
            (c.state :: ControllerState).cloud = nil
        end
        me:set_lifecycle("destroy")
    end
end

local ACTIONS = { [WAIT] = wait, [RAINING] = raining, [DONE] = done, [FADE] = fade }

local function update(me: Object)
    local a = ACTIONS[me.action]
    if not a then
        error(string.format("RainyDay's cloud: action %#x reads past 0x080D9804", me.action))
    end
    a(me)
end

-- 0x080D97CC: over its panel, 60 pixels up, shown; the first update at once.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, Z)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 0, 0xFF
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.visible = true
    me.lifecycle = "update"
    update(me)
end

rainyday.cloud = new.kind {
    id = "rainyday/cloud",
    pool = "attack",
    state = { scan_x = "i32", scan_y = "i32", dir = "i32", rain = "object", controller = "object" },
    -- 0x080D97A8: by its lifecycle, then a sprite step.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            update(me)
        else
            me:free()
            return
        end
        me:update_sprite()
    end,
}

-- ---- The controller -------------------------------------------------------------------------------------------------

-- The enemy navis (0x080E9180).
local TARGETS = { [0] = 0x0400_0000, [1] = 0x0800_0000 }

export type ControllerState = {
    -- The Atk+ bonus (+0x32): it adds to the cloud's damage word.
    bonus: number,
    -- The cloud, while it is out (+0x60, which it clears).
    cloud: Object?,
}

-- 0x080E9128: the first panel with an enemy navi, by columns from the panel ahead, rows 1 to 3.
local function target(me: Object): (number?, number?)
    local x = me.panel_x + me.facing
    local mask = TARGETS[me.alliance]
    while field.valid(x, 1) do
        for y = 1, 3 do
            if bit32.btest(field.flags(x, y), mask) then
                return x, y
            end
        end
        x += me.facing
    end
    return nil, nil
end

-- 0x080E90D0.
local function effect(me: Object)
    local s = me.state :: ControllerState
    if me.phase_init == 0 then
        local x, y = target(me)
        if x == nil then
            dimming_chips.done(me)
            return
        end
        -- 0x080D9940: the cloud (its position the spawner's registers until its init places it), running while
        -- dimmed.
        local o = battle.spawn(rainyday.cloud, Vec3.new(y :: number, me.element, 0))
        if o then
            o.panel_x, o.panel_y = x, y :: number
            o.element = me.element
            o:set_damage_word(dimming_chips.damage_plus(me, s.bonus))
            o.alliance, o.flip = me.alliance, me.flip
            o.related1 = me
            o.run_while_dimmed = true
            ;(o.state :: CloudState).controller = me
            s.cloud = o
        end
        me.phase_init = 4
    end
    if s.cloud == nil then
        dimming_chips.done(me)
    end
end

rainyday.controller = new.kind {
    id = "rainyday/controller",
    pool = "effect",
    state = { bonus = "u16", cloud = "object" },
    -- 0x080E908C.
    update = dimming_chips.phases { name = "RainyDay", effect = effect },
}

-- 0x080E9106: the chip's `dimming` hook.
function rainyday.hook(user: Object, spec: DimmingChipSpec): Object?
    local o = dimming_chips.spawn(rainyday.controller, user, spec)
    if o then
        (o.state :: ControllerState).bonus = spec.bonus
    end
    return o
end

return rainyday
