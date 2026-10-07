--!strict
-- EXE4's towers (attack object 0x92, 0x080DF68C: EXE4's own), which the towers' controllers (lib/towers) raise:
-- AquaUp's aqua tower and GreenWd's wood tower, each its own (`tower.make`) with its look, the row of 0x080DF674 its
-- spawn names (12 bytes: the sprite, three animations and their ticks, 0xFF for none, the hit modifier and the sound).
-- On its panel, unturned, a thrown body hitting navis once (its region the collision's single panel, gone after its
-- first hit), it rises with its sound, holds and sinks in its look's animations and ticks, then goes.

local collision = require("../rules/collision")
local sparks = require("../lib/sparks")

local tower = {}

-- A tower's look: its key, the sprite, then each step's animation (nil: the one before goes on) and ticks (a step
-- lasts its ticks and one more), the hit modifier and the sound as it rises.
type Look = {
    id: string,
    sprite: SpriteAsset,
    steps: { { anim: number?, ticks: number } },
    hit_mod: number,
    sound: Sound,
}

local RISE, HOLD, SINK = 0x0, 0x4, 0x8

-- A tower of `look`: its spawn (0x080DF7D0), a tower on (x, y) with the element and damage word, of `owner`'s side.
-- (Its position is the registers until its init places it.)
function tower.make(look: Look): (owner: Object, x: number, y: number, element: number, damage: number) -> ()
    -- 0x080DF754, 0x080DF77C, 0x080DF7A6: a step of the look: on its first tick its ticks, and the sound rising (its
    -- animation the init's) or the step's animation after; past them the next step, past the last gone.
    local function step(me: Object, i: number)
        local st = look.steps[i]
        if me.phase_init == 0 then
            if i == 1 then
                battle.play_sound(look.sound)
            elseif st.anim then
                me.anim = st.anim
            end
            me.timer = st.ticks
            me.phase_init = 4
        end
        local t = me.timer - 1
        if t >= 0 then
            me.timer = t
            return
        end
        me.timer = 0xFFFF
        if i < #look.steps then
            me.action = i * 4
            me.phase, me.phase_init = 0, 0
        else
            me.collision:set_region(nil)
            me:set_lifecycle("destroy")
        end
    end

    -- 0x080DF71A: the hits against it resolve and spark (its region gone after it hits), the step, present.
    local function tick(me: Object)
        local c = me.collision
        c:remove()
        me:hit_spark()
        if c.hit_flags ~= 0 then
            c:set_region(nil)
        end
        local a = me.action
        if a == RISE then
            step(me, 1)
        elseif a == HOLD then
            step(me, 2)
        elseif a == SINK then
            step(me, 3)
        else
            error(string.format("a tower's action %#x reads past its table", a))
        end
        c:present()
    end

    -- 0x080DF6B0: its sprite (with its shadow drawn with it) on its panel, its first animation requested (the loaded
    -- one 0xFF, which the init's own load and step take: the update's sprite step loads the animation); a thrown body
    -- hitting navis with its modifier and the element's spark; then its first tick.
    local function init(me: Object)
        local sprite = me.sprite
        sprite:load(look.sprite)
        sprite.shadow = "with_sprite"
        me.visible = true
        me.anim, me.anim_loaded = look.steps[1].anim :: number, 0xFF
        me:set_coordinates_from_panel()
        if not me:create_collision() then
            me:free()
            return
        end
        me:setup_collision(collision.thrown, collision.hits_navis, look.hit_mod)
        me.collision:set_hit_effect(sparks.of_element(me.element))
        me.collision:present()
        me:set_lifecycle("update")
        tick(me)
    end

    local kind = new.kind {
        id = look.id,
        pool = "attack",
        -- 0x080DF68C: then the sprite, even while dimmed (0x08014446).
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                tick(me)
            else
                me:destroy()
            end
            me:update_sprite_while_dimmed()
        end,
    }

    return function(owner: Object, x: number, y: number, element: number, damage: number)
        local o = battle.spawn(kind, Vec3.new(y, element, 0))
        if o then
            o.panel_x, o.panel_y = x, y
            o.element = element
            o.alliance = owner.alliance
            o:set_damage_word(damage)
        end
    end
end

return tower
