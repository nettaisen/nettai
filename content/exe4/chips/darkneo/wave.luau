--!strict
-- DarkNeo's wave (EXE4's attack object #0xA2, 0x080E128C), what its user's image fires (./image): the bug charger's
-- sprite (0c-43, its first part drawn with it) flying 10 pixels a tick along its row from its panel, at the height it
-- is spawned at; an attack hitting navis (hit modifier 3, the plain spark) once a panel: a hit takes its region until
-- it is over another panel. Every fourth tick it leaves an afterimage (8 ticks, tinted 0xA94A). Gone past the field.
-- It runs while dimmed. Its sound 0x88.

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local wave = {}

type State = {
    -- The panel (x | y << 8) where it last hit something (its +0x60).
    hit_panel: number,
}

local SPRITE = asset.sprite("bug-charger")
local SOUND = asset.sound("kendo-draw")
local SPEED = 0xA_0000
local HIT_MOD = 3
-- An afterimage every fourth tick: its lifetime and tint.
local TRAIL_EVERY, TRAIL_TICKS, TRAIL_SHADER = 4, 8, 0xA94A

-- The panel as the game's halfword (x | y << 8).
local function panel_word(me: Object): number
    return me.panel_x + bit32.lshift(me.panel_y, 8)
end

-- 0x080E13CE: an afterimage of it where it is, by its side (0x080E5020: its shadow at its height).
local function trail(me: Object)
    battle.afterimage(me, me.pos, {
        sprite = SPRITE,
        anim = me.anim,
        flip = me.alliance,
        lifetime = TRAIL_TICKS,
        color_shader = TRAIL_SHADER,
    })
end

-- 0x080E1328: last tick's hits (a hit takes its region and notes the panel); its flight (0x080E1384); its region back
-- over another panel; presented while over the field, else gone.
local function tick(me: Object)
    local s = me.state :: State
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 then
        c:set_region(nil)
        s.hit_panel = panel_word(me)
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t == 0 then
        me.timer = TRAIL_EVERY
        trail(me)
    end
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x + me.vel.x), p.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if c.region == nil and panel_word(me) ~= s.hit_panel then
        c:set_region(regions.single)
    end
    if field.valid(me.panel_x, me.panel_y) then
        c:present()
        return
    end
    c:set_region(nil)
    me:set_lifecycle("destroy")
end

-- 0x080E12B0.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    -- sprite_noShadow: the first part is drawn with the sprite.
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.vel = Vec3.new(me.facing * SPEED, me.vel.y, me.vel.z)
    me:set_coordinates_from_panel()
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.attack, collision.hits_navis, HIT_MOD)
    me.collision:set_hit_effect(sparks.plain)
    me.collision:present()
    me.timer = TRAIL_EVERY
    me:set_lifecycle("update")
    tick(me)
    battle.play_sound(SOUND)
end

wave.kind = new.kind {
    id = "darkneo/wave",
    pool = "attack",
    state = { hit_panel = "u16" },
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

-- 0x080E13A8: a wave of `owner`'s side on (x, y) at height `z`, null, with the damage word, running while dimmed. (Its
-- position is the spawner's registers, the row, the element and the height, until its init places it; the height
-- stays.)
function wave.spawn(owner: Object, x: number, y: number, z: number, damage: number): Object?
    local o = battle.spawn(wave.kind, Vec3.new(y, 0, z))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = 0
        o:set_damage_word(damage)
        o.alliance = owner.alliance
        o.run_while_dimmed = true
    end
    return o
end

return wave
