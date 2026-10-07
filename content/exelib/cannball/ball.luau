--!strict
-- The cannonball, as EXE5 and EXE4 share it (EXE5's attack object #0x35, 0x080CB5C8, CannBall's, thrown by action 0x12's
-- subtype 4, 0x080CB7D4; EXE4's #0x35, 0x080D384C, Ball's, thrown by action 0x09's variant 4, 0x080D3A58; EXE6 keeps the
-- code, `sub_80CA938`, and no chip throws it): lobbed in a 50-tick arc at the panel three ahead, it can hit on its way
-- (its region set once it lands); landing on a solid panel it hits it, breaks it (cracks it while something stands on
-- it) with an impact and a crack's sound; off solid ground it ends in a puff, and a hit ends it at once. Each game
-- makes its own (`ball.make`) with its look: its kind, sprite, effects, sound and collision types. (Its kind, Param1,
-- is the chips' 0: the original's kind 2, a plain attack bursting over the column, no chip throws, and kinds 1 and 3
-- differ from 0 only in their region, EXE5's 0x080CB7A0, EXE4's 0x080D3A24.)

local regions = require("../regions")
local trajectory = require("../trajectory")

local ball = {}

type State = {
    -- The panel it aims at (Param3 and Param4).
    target_x: number,
    target_y: number,
}

local FLIGHT_TICKS, GRAVITY = 50, -0x3000
local HIT_MOD = 3
-- Its sprite's animation and palette (EXE5's 0x080CB5EC).
local ANIM, PALETTE = 1, 4
-- Its target: three panels ahead.
local REACH = 3

-- A game's cannonball, aimed over its sine table.
function ball.make(look: CannonballLook, aim: Trajectory): Cannonball
    -- EXE5's 0x080CB6BC: fly to the target panel; land there.
    local function fly(me: Object, s: State)
        if me.phase_init == 0 then
            local tx, ty = field.center(s.target_x, s.target_y)
            me.vel = aim.in_ticks(me.pos, tx, ty, 0, FLIGHT_TICKS, GRAVITY)
            me.timer = FLIGHT_TICKS
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = bit32.band(t, 0xFFFF)
        if t >= 0 then
            local p, v = me.pos, me.vel
            me.pos = Vec3.new(p.x + v.x, p.y + v.y, int.i32(p.z + v.z))
            me.vel = Vec3.new(v.x, v.y, int.i32(v.z + GRAVITY))
            me:set_panel_from_coordinates()
            me:update_collision_panels()
            return
        end
        me:set_coordinates_from_panel()
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y, 0)
        local x, y = me.panel_x, me.panel_y
        if not field.solid(x, y) then
            battle.effect(me.pos, look.puff)
            me:set_lifecycle("destroy")
            return
        end
        -- (Its region once landed, by its kind: one panel.)
        me.collision:set_region(regions.single)
        field.break_panel(x, y)
        battle.effect(me.pos, look.impact)
        battle.play_sound(look.crack)
        me.action = 4
        me.phase, me.phase_init = 0, 0
    end

    -- EXE5's 0x080CB65A: resolve last tick's hits; the battle over, a puff; a hit, gone; else fly, then go.
    local function tick(me: Object)
        me.collision:remove()
        me:hit_spark()
        if battle.over() then
            battle.effect(me.pos, look.puff)
            me.collision:set_region(nil)
            me:set_lifecycle("destroy")
            return
        end
        if me.collision.hit_flags ~= 0 then
            me.collision:set_region(nil)
            me:set_lifecycle("destroy")
            return
        end
        local a = me.action
        if a == 0 then
            fly(me, me.state :: State)
        elseif a == 4 then
            -- EXE5's 0x080CB7A4.
            me.collision:set_region(nil)
            me.visible = false
            me:set_lifecycle("destroy")
        else
            error(string.format("the cannonball's action %#x reads past its table", a))
        end
        me.collision:present()
    end

    local kind = new.kind {
        id = look.id,
        pool = "attack",
        state = { target_x = "u8", target_y = "u8" },
        -- EXE5's 0x080CB5C8.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                -- EXE5's 0x080CB5EC.
                local sprite = me.sprite
                sprite:load(look.sprite)
                sprite.shadow = "ground"
                me.visible = true
                me.anim, me.anim_loaded = ANIM, ANIM
                sprite:set_animation(ANIM)
                sprite.palette = PALETTE
                me:set_panel_from_coordinates()
                if not me:create_collision() then
                    me:free()
                    return
                end
                me:setup_collision(look.is, look.hits, HIT_MOD)
                me.collision:set_hit_effect(nil)
                me.collision:set_region(nil)
                me.collision:present()
                me:set_lifecycle("update")
                tick(me)
            elseif l == "update" then
                tick(me)
            else
                me:destroy()
                return
            end
            me:update_sprite()
        end,
    }

    -- EXE5's 0x080CB7D4: a thrower of cannonballs at the panel three ahead of the user.
    local function thrower(): Thrower
        return {
            throw = function(user: Object, pos: Vec3, damage: number): Object?
                local o = battle.spawn(kind, pos)
                if o then
                    o.alliance = user.alliance
                    o.element = 0
                    o:set_damage_word(damage)
                    local s = o.state :: State
                    s.target_x, s.target_y = user.panel_x + REACH * user.facing, user.panel_y
                end
                return o
            end,
        }
    end

    return { kind = kind, thrower = thrower }
end

return ball
