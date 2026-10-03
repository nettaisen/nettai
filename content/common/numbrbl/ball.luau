--!strict
-- A NumbrBl ball (attack object #0x91 in both games: BN6's `sub_80D7278`,
-- docs/engine/dimming-chip-effects.md §15; BN5's 0x080D895C), made for a
-- game's look (`ball.make`): its thrower sets it down in front of him; it
-- sits, waits, then rolls down the row 3 pixels a tick until it hits
-- something (an explosion) or leaves the field. Its hit is live from its
-- first tick, even while it waits.
--
-- The games part: BN6's sits 15 ticks, BN5's 30 (with another sound), and
-- BN5's hides its sprite's parts 2 to 21 each tick it waits and rolls.


local ball = {}

export type State = {
    -- The ticks it waits before rolling, less one (Param2).
    wait: number,
}

-- Rolling 3 pixels a tick; hit modifier 3.
local SPEED, HIT_MOD = 0x3_0000, 3
local ANIM_ROLL = 1
-- Gone once its X is more than 150 pixels off the field's middle either
-- way (the unsigned test `x + 150 > 300`).
local REACH = 0x96

local function next_action(me: Object, a: number)
    me.action, me.phase, me.phase_init = a, 0, 0
end

-- A ball of `look`'s game: its kind and `spawn`.
function ball.make(look: NumberBallLook): NumberBall
    -- BN5's 0x080D8ABA: the parts it hides.
    local function hide_parts(me: Object)
        local hidden = look.hidden_parts
        if hidden then
            me.sprite.hidden_parts = hidden
        end
    end

    -- `sub_80D7358` (BN5's 0x080D8A34): sit its ticks, the sound as it is
    -- set down.
    local function sit(me: Object)
        if me.phase_init == 0 then
            me.timer = look.sit_ticks
            battle.play_sound(look.set_sound)
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t < 0 then
            next_action(me, 4)
        end
    end

    -- `sub_80D7382` (BN5's 0x080D8A5E): rolling's animation and shadow; the
    -- wait.
    local function ready(me: Object)
        hide_parts(me)
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

    -- `sub_80D73AC` (BN5's 0x080D8A8C): roll on; gone past the field.
    local function roll(me: Object)
        hide_parts(me)
        local p = me.pos
        me.pos = Vec3.new(p.x + me.vel.x, p.y, p.z)
        me:set_panel_from_coordinates()
        me:update_collision_panels()
        if int.u32(int.asr(me.pos.x, 16) + REACH) > 2 * REACH then
            me.collision:set_region(nil)
            me:set_lifecycle("destroy")
        end
    end

    -- `sub_80D730A` (BN5's 0x080D89E6): resolve last tick's hits; a hit
    -- ends it with an explosion where it is; else its action, and present.
    local function tick(me: Object)
        local c = me.collision
        c:remove()
        me:hit_spark()
        if c.hit_flags ~= 0 then
            battle.effect(me.pos, look.explosion)
            battle.play_sound(look.hit_sound)
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
            error(string.format("NumbrBl's ball: action %#x reads past its table", a))
        end
        c:present()
    end

    -- `sub_80D729C` (BN5's 0x080D8980): on its panel (its Z the spawner's),
    -- no shadow yet, its collision live; on at once. (Without a collision
    -- slot it is freed.)
    local function init(me: Object)
        local sprite = me.sprite
        sprite:load(look.sprite)
        sprite.shadow = "with_sprite"
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
        me:setup_collision(look.is, look.hits, HIT_MOD)
        me.collision:present()
        me:set_lifecycle("update")
        tick(me)
    end

    local kind = define.kind {
        id = look.id,
        pool = "attack",
        state = { wait = "u8" },
        -- `sub_80D7278` (BN5's 0x080D895C): by state, then the sprite
        -- (stepping while dimmed).
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

    -- `sub_80D73D6` (BN5's 0x080D8AC4): a ball on (`x`, `y`) at height `z`,
    -- of `thrower`'s side and facing, with its element and damage word,
    -- waiting `wait` ticks (plus one) before it rolls, running while
    -- dimmed. (Its X and Y are the spawner's registers until its init.)
    local function spawn(thrower: Object, x: number, y: number, z: number, wait: number): Object?
        local o = battle.spawn(kind, Vec3.new(0, 0, z))
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

    return { kind = kind, spawn = spawn }
end

return ball
