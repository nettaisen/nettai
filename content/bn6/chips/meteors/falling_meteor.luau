--!strict
-- A falling meteor (the original's attack object #0x56, `sub_80CF3BE`;
-- docs/engine/dimming-chip-effects.md §6): marked on its panel (./marker),
-- it waits 30 ticks high up behind it, falls onto it in 16, and if the
-- panel is solid or holds a body, bursts with a one-tick hit and the camera
-- shaking. Meteors (./chip) drops thirty (running while dimmed); the
-- instant chips' meteor shower (lib/instant/meteor_shower) uses it too,
-- dropping them one at a time.

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local marker = require("./marker")
local regions = require("../../lib/regions")
local sparks = require("../../lib/sparks")

local falling_meteor = {}

local SPRITE = asset.sprite("meteor")
local ANIM = 0
-- It starts 256 pixels behind and above its panel, and falls 16 forward
-- and 16 down a tick.
local START, SPEED = 0x100_0000, 0x10_0000
local WAIT, FALL = 0x1E, 0x10
local SOUND_FALL = asset.sound("justice-one")
local SOUND_LAND = asset.sound("hit-bomb-1")
-- What it is and reaches (`byte_80CF3AC`: every row's): thrown, hitting
-- navis, hit modifier 3.
local IS, HITS, HIT_MOD = collision.thrown, collision.hits_navis, 3
-- It bursts where the panel is solid or holds a body.
local BURSTS_ON = 0x0F80_0010
-- The camera shake it lands with (every row's: 1, for 15 ticks).
local SHAKE, SHAKE_TICKS = 1, 0xF

-- `sub_80CF4C8`: 30 ticks up there.
local function wait(me: Object)
    local t = me.timer - 1
    me.timer = t
    if t ~= 0 then
        return
    end
    me.timer = FALL
    battle.play_sound(SOUND_FALL)
    me.action = 4
end

-- `sub_80CF4E6`: fall; on the panel, burst if it is solid or holds a body.
local function fall(me: Object)
    local t = me.timer - 1
    me.timer = t
    if t ~= 0 then
        local p, v = me.pos, me.vel
        me.pos = Vec3.new(p.x + v.x, p.y, p.z - v.z)
        return
    end
    me:set_coordinates_from_panel()
    me.pos = Vec3.new(me.pos.x, me.pos.y, 0)
    if bit32.btest(field.flags(me.panel_x, me.panel_y), BURSTS_ON) then
        battle.effect(me.pos, effects.burst)
        battle.play_sound(SOUND_LAND)
        battle.shake_camera(SHAKE, SHAKE_TICKS)
        me.collision:set_region(regions.single)
    end
    me.action = 8
    me.visible = false
end

-- `sub_80CF554`: the tick after (its hit resolved): gone. (Every row
-- leaves the panel as it is.)
local function landed(me: Object)
    me.visible = false
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- `sub_80CF488`: resolve last tick's hits, the action, present.
local function tick(me: Object)
    me.collision:remove()
    me:hit_spark()
    local a = me.action
    if a == 0 then
        wait(me)
    elseif a == 4 then
        fall(me)
    elseif a == 8 then
        landed(me)
    else
        error(string.format("a falling meteor's action %#x reads past its table", a))
    end
    me:update_sprite()
    me.collision:present()
end

-- `sub_80CF3DC`: behind and above its panel, marked; its collision on no
-- panel yet. (The update first runs next tick.)
local function init(me: Object)
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    -- sprite_noShadow: the first part is drawn with the sprite.
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = ANIM, ANIM
    sprite:set_animation(ANIM)
    sprite:step()
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    local d = me.facing
    me.vel = Vec3.new(SPEED * d, me.vel.y, SPEED)
    me.pos = Vec3.new(me.pos.x - START * d, me.pos.y, START)
    me.timer = WAIT
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(IS, HITS, HIT_MOD)
    me.collision:set_hit_effect(sparks.of_element(me.element))
    me.collision:set_region(nil)
    me.collision:present()
    marker.spawn(me, me.panel_x, me.panel_y, marker.meteor, regions.single)
    me.lifecycle = "update"
end

falling_meteor.kind = define.kind {
    id = "meteors/falling-meteor",
    pool = "attack",
    -- `sub_80CF3BE`.
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

-- `sub_80CF594`: a meteor on (x, y) with `element` and the damage word, of
-- `owner`'s side and facing; `while_dimmed` (`sub_80CF5B2`, Meteors'): it
-- runs while dimmed. Its position is `pos` (the spawner's registers) until
-- its init places it.
function falling_meteor.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    damage: number,
    while_dimmed: boolean,
    pos: Vec3?
): Object?
    local o = battle.spawn(falling_meteor.kind, pos)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = element
    o:set_damage_word(damage)
    o.alliance, o.flip = owner.alliance, owner.flip
    if while_dimmed then
        o.run_while_dimmed = true
    end
    return o
end

return falling_meteor
