--!strict
-- The FlmLines' flame (attack object #0x19, 0x080CFF80: EXE4's own): a flame rising from a solid panel (its sprite
-- 10-0a, its shadow on the ground; the sound 0x9D), a fire attack hitting navis with hit modifier 3 and the fire spark.
-- It rises for 3 ticks (animation 0) with no region, burns for its ticks (animation 1, the panel's region), then dies
-- down for 3 (animation 2) with none and goes; a hit takes it off the panel at once.

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local flame = {}

local SPRITE = asset.sprite("flame-line")
local SOUND = asset.sound("flame-line")
-- A panel it can stand on: a solid one (0x08009ECC).
local SOLID = 0x10
local HIT_MOD = 3
local RISE_TICKS, SINK_TICKS = 3, 3

-- Its actions.
local RISE, BURN, SINK = 0x0, 0x4, 0x8

type State = {
    -- The ticks it burns (its second parameter, +5: the chip's).
    burns: number,
}

-- 0x080D0040: rising, 3 ticks; then it burns.
local function rise(me: Object)
    if me.phase_init == 0 then
        me.timer = RISE_TICKS
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t > 0 then
        return
    end
    me.action = BURN
    me.phase, me.phase_init = 0, 0
end

-- 0x080D0062: burning (animation 1) on its panel for its ticks; then off it, dying down.
local function burn(me: Object)
    if me.phase_init == 0 then
        me.collision:set_region(regions.single)
        me.anim = 1
        me.timer = (me.state :: State).burns
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t > 0 then
        return
    end
    me.collision:set_region(nil)
    me.action = SINK
    me.phase, me.phase_init = 0, 0
end

-- 0x080D0092: dying down (animation 2), 3 ticks; then gone.
local function sink(me: Object)
    if me.phase_init == 0 then
        me.collision:set_region(nil)
        me.anim = 2
        me.timer = SINK_TICKS
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t > 0 then
        return
    end
    me:set_lifecycle("destroy")
end

-- 0x080D0008: the hits against it resolve and spark (a hit takes it off its panel), its action, present.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    local a = me.action
    if a == RISE then
        rise(me)
    elseif a == BURN then
        burn(me)
    elseif a == SINK then
        sink(me)
    else
        error(string.format("a flame's action %#x reads past its table (0x080D0034)", a))
    end
    c:present()
end

-- 0x080CFFA4: on a solid panel, its sprite (its shadow on the ground) in its first animation requested, on its panel;
-- an attack hitting navis with the fire spark, on no panel yet; its sound; then its first tick.
local function init(me: Object)
    if not field.check(me.panel_x, me.panel_y, SOLID, 0) then
        me:free()
        return
    end
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 0, 0xFF
    me.visible = true
    me:set_coordinates_from_panel()
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.attack, collision.hits_navis, HIT_MOD)
    local c = me.collision
    c:set_hit_effect(sparks.fire)
    c:set_region(nil)
    c:present()
    battle.play_sound(SOUND)
    me:set_lifecycle("update")
    tick(me)
end

local kind = new.kind {
    id = "flmline/flame",
    pool = "attack",
    state = { burns = "u8" },
    -- 0x080CFF80: then the sprite.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        me:update_sprite()
    end,
}

-- 0x080D00B8: a flame of `user`'s on (x, y) with the attack's element and damage word, burning `burns` ticks; its
-- owner the user. (Its position is the spawner's registers until its init places it: Y, the element and 0.)
function flame.spawn(user: Object, x: number, y: number, element: number, damage: number, burns: number)
    local o = battle.spawn(kind, Vec3.new(y, element, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance = user.alliance
        o.related1 = user;
        (o.state :: State).burns = burns
    end
end

return flame
