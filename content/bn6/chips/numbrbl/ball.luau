--!strict
-- A NumbrBl ball (the original's attack object #0x91, `sub_80D7278`;
-- docs/engine/dimming-chip-effects.md §15): NumberMan (./numberman) sets
-- it down in front of him; it waits, then rolls down the row 3 pixels a
-- tick until it hits something (an explosion) or leaves the field. Its
-- hit is live from its first tick, even while it waits.

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")

local ball = {}

export type State = {
    -- The ticks it waits before rolling, less one (Param2).
    wait: number,
}

local SPRITE = asset.sprite("number-ball")
local SOUND_SET = asset.sound("place")
local SOUND_HIT = asset.sound("damage")
-- An attack hitting navis, hit modifier 3; rolling 3 pixels a tick.
local IS, HITS, HIT_MOD = collision.attack, collision.hits_navis, 3
local SPEED = 0x3_0000
-- The ticks it sits before its wait (the first sets the count).
local SIT = 0xE
local ANIM_ROLL = 1
-- Gone once its X is more than 150 pixels off the field's middle either
-- way (the unsigned test `x + 150 > 300`).
local REACH = 0x96

local function next_action(me: Object, a: number)
    me.action, me.phase, me.phase_init = a, 0, 0
end

-- `sub_80D7358`: 15 ticks, the sound as it is set down.
local function sit(me: Object)
    if me.phase_init == 0 then
        me.timer = SIT
        battle.play_sound(SOUND_SET)
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        next_action(me, 4)
    end
end

-- `sub_80D7382`: rolling's animation and shadow; the wait.
local function ready(me: Object)
    if me.phase_init == 0 then
        me.anim = ANIM_ROLL
        me.sprite.shadow = "ground"
        me.timer = (me.state :: State).wait
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        next_action(me, 8)
    end
end

-- `sub_80D73AC`: roll on; gone past the field.
local function roll(me: Object)
    local p = me.pos
    me.pos = Vec3.new(p.x + me.vel.x, p.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if int.u32(int.asr(me.pos.x, 16) + REACH) > 2 * REACH then
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
    end
end

-- `sub_80D730A`: resolve last tick's hits; a hit ends it with an
-- explosion where it is; else its action, and present.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 then
        battle.effect(me.pos, effects.explosion)
        battle.play_sound(SOUND_HIT)
        c:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    local a = me.action
    if a == 0 then
        sit(me)
    elseif a == 4 then
        ready(me)
    elseif a == 8 then
        roll(me)
    else
        error(string.format("NumbrBl's ball: action %#x reads past off_80D734C", a))
    end
    c:present()
end

-- `sub_80D729C`: on its panel (its Z the spawner's), no shadow yet, its
-- collision live; on at once. (Without a collision slot it is freed.)
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "hidden"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0xFF
    sprite:set_animation(0)
    sprite:step()
    me:set_coordinates_from_panel()
    me.vel = Vec3.new(me.facing * SPEED, me.vel.y, me.vel.z)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(IS, HITS, HIT_MOD)
    me.collision:present()
    me:set_lifecycle("update")
    tick(me)
end

ball.kind = define.kind {
    id = "numbrbl/ball",
    pool = "attack",
    state = { wait = "u8" },
    -- `sub_80D7278`: by state, then the sprite (stepping while dimmed).
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
        me:update_sprite_while_dimmed()
    end,
}

-- `sub_80D73D6`: a ball on (`x`, `y`) at height `z`, of `thrower`'s side
-- and facing, with its element and damage word, waiting `wait` ticks
-- (plus one) before it rolls, running while dimmed. (Its X and Y are the
-- spawner's registers until its init.)
function ball.spawn(thrower: Object, x: number, y: number, z: number, wait: number): Object?
    local o = battle.spawn(ball.kind, Vec3.new(0, 0, z))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = thrower.element
    o.alliance, o.flip = thrower.alliance, thrower.flip
    o:set_damage_word(bit32.bor(thrower.damage, bit32.lshift(thrower.stamina, 16)))
    o.run_while_dimmed = true
    (o.state :: State).wait = wait
    return o
end

return ball
