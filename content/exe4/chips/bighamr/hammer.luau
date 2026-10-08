--!strict
-- The hammers (EXE4's attack object #0x4A, 0x080D68CC: EXE4's own): BigHamr's and GodHammr's. One of its side's field
-- objects (an obstacle with 100 HP, no lifetime) in front of its user: it appears (sound 0xB0), waits 10 ticks, winds
-- up for 24 ticks (BigHamr's blinking between its animations 1 and 0), then swings (sound 0x122, its animation 2) and
-- 5 ticks later lands on the panel ahead: what the landing does is each chip's own (its `land`), a solid panel
-- shaking the camera for 30 ticks (BigHamr1's by 1, BigHamr2's by 2, BigHamr3's and GodHammr's by 3) with the
-- landing's sound (0xA2: `hammer.shake`). 60 ticks later it goes. Whatever ends it (broken, or after its swing), it leaves in an explosion with its
-- sound (0x080D6B42). What differs between the chips is its look (`hammer.make`): its palette, the animation it
-- stands in, whether it blinks as it winds up, the shake, its landing; each chip its own kind, the original's one
-- object compat's.
--
-- An obstacle: its update runs the obstacle framework's steps (the `obstacle` service) around its own actions
-- (0x080D6974: its hits, its reaction with its table 0x080D698C, its collision; its sprite after each state).

local collision = require("../../rules/collision")
local panels = require("@exelib/panels")

local hammer = {}

export type Look = {
    -- The kind's id.
    id: string,
    -- Its palette (0x080D692C: BigHamr's 0, 4, 8, GodHammr's 10) and the animation it stands in (BigHamr's 1,
    -- GodHammr's 0).
    palette: number,
    stand_anim: number,
    -- As it winds up it blinks between its animations 1 and 0 (BigHamr's, 0x080D6A24), else it stands still.
    blinks: boolean,
    -- What its landing does on the panel ahead (`x`, `y`), solid or not, with its damage word.
    land: (me: Object, x: number, y: number, solid: boolean) -> (),
}

export type Hammer = {
    kind: Kind,
    -- 0x080D6BCC: on panel (`x`, `y`), on `owner`'s side (not its facing), of `element` and the damage word: one of
    -- the side's field objects; it runs while the battle is dimmed.
    spawn: (owner: Object, x: number, y: number, element: number, damage: number) -> Object?,
}

local SPRITE = asset.sprite("big-hammer")
local APPEAR_SOUND = asset.sound("appear")
local SWING_SOUND = asset.sound("hammer-swing")
local LAND_SOUND = asset.sound("metal-punch")
local LEAVE_SOUND = asset.sound("hit-bomb-1")
-- 0x080D6B58: effect row 0, 14-00, where it stands.
local EXPLOSION = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0 }

local HP = 100
local HIT_MOD = 3
-- The waits: before winding up, winding up, the swing, after the landing.
local WAIT, WIND_UP, SWING, AFTER = 10, 24, 5, 60
local SHAKE_TICKS = 30
-- The animations: standing still, swinging.
local ANIM_STILL, ANIM_SWING = 0, 2

-- The actions (the framework's numbering: the kind's own from 8).
local APPEAR, DESTROYED, WAITING, SWINGING = 0, 2, 8, 9

-- Its hit on its landing's panels (0x080CD7E2 with r3 0 and r7 3): its element, no status, hit modifier 3.
function hammer.hit(me: Object, x: number, y: number, region: Region, is: CollisionType, spark: Spark?)
    battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = me.element,
        z = 0,
        region = region,
        hit_effect = spark,
        target = collision.hits_navis,
        self_type = is,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = HIT_MOD,
    })
end

-- 0x080D6AB6: landing on a solid panel, the camera shakes by `magnitude` for 30 ticks, with the landing's sound.
function hammer.shake(magnitude: number)
    battle.shake_camera(magnitude, SHAKE_TICKS)
    battle.play_sound(LAND_SOUND)
end

-- 0x080D6B74 (BigHamr's): two of the other side's solid panels cracked, picked at random (the panels but its own
-- meeting its side's condition, 0x080D6BBC: the other side's solid panels, not metal; shuffled with as many swaps as
-- panels, `sub_8000C72`).
local CRACKED = { [0] = { require = 0x30, forbid = 0x20000 }, [1] = { require = 0x10, forbid = 0x20020 } }

function hammer.crack_two(me: Object)
    local cond = CRACKED[me.alliance]
    local list = panels.filtered_except(me, cond.require, cond.forbid)
    local n = #list
    if n == 0 then
        return
    end
    panels.shuffle(list, n, n)
    for i = 1, math.min(n, 2) do
        field.crack(panels.x(list[i]), panels.y(list[i]))
    end
end

function hammer.make(look: Look): Hammer
    -- 0x080D6B42: gone in an explosion.
    local function destroyed(me: Object)
        me.visible = false
        me.collision:set_region(nil)
        battle.effect(me.pos, EXPLOSION)
        battle.play_sound(LEAVE_SOUND)
        obstacle.unregister(me)
        me:unreserve_panel(me.future_panel_x, me.future_panel_y)
        me:set_lifecycle("destroy")
    end

    -- 0x080D69AC: it appears, then waits.
    local function appear(me: Object)
        battle.play_sound(APPEAR_SOUND)
        me:set_action(obstacle.action_byte(me, WAITING))
    end

    -- 0x080D69BE: 10 ticks, then swing.
    local function wait(me: Object)
        if me.phase_init == 0 then
            me.timer = WAIT
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t < 0 then
            me:set_action(obstacle.action_byte(me, SWINGING))
        end
    end

    -- 0x080D6A04: wind up for 24 ticks (blinking: its animation 1 while the timer's bit 1 is clear, else 0).
    local function wind_up(me: Object)
        if me.phase_init == 0 then
            if look.blinks then
                me.anim = ANIM_STILL
            end
            me.timer = WIND_UP
            me.phase_init = 4
        end
        if look.blinks then
            me.anim = if bit32.btest(me.timer, 2) then ANIM_STILL else look.stand_anim
        end
        local t = me.timer - 1
        me.timer = t
        if t < 0 then
            if look.blinks then
                me.anim = ANIM_STILL
            end
            me.phase, me.phase_init = 4, 0
        end
    end

    -- 0x080D6A48: a tick.
    local function pause(me: Object)
        if me.phase_init == 0 then
            me.timer = 0
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t < 0 then
            me.phase, me.phase_init = 8, 0
        end
    end

    -- 0x080D6A66: the swing; 5 ticks later it lands on the panel ahead.
    local function swing(me: Object)
        if me.phase_init == 0 then
            battle.play_sound(SWING_SOUND)
            me.anim = ANIM_SWING
            me.timer = SWING
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t >= 0 then
            return
        end
        local x, y = me.panel_x + panels.side_direction(me.alliance), me.panel_y
        look.land(me, x, y, field.solid(x, y))
        me.phase, me.phase_init = 0xC, 0
    end

    -- 0x080D6B20: 60 ticks, then it goes.
    local function after(me: Object)
        if me.phase_init == 0 then
            me.timer = AFTER
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t < 0 then
            me:set_action(obstacle.action_byte(me, DESTROYED))
        end
    end

    -- 0x080D69E0 by its phase (0x080D69F4).
    local function swinging(me: Object)
        local phase = me.phase
        if phase == 0 then
            wind_up(me)
        elseif phase == 4 then
            pause(me)
        elseif phase == 8 then
            swing(me)
        elseif phase == 0xC then
            after(me)
        else
            error(string.format("a hammer's swinging phase %#x reads past 0x080D69F4", phase))
        end
    end

    -- Its action table (0x080D698C): its own actions and the framework's shared ones.
    local ACTIONS: { [number]: (Object) -> () } = {
        [APPEAR] = appear,
        [1] = obstacle.return_to_idle,
        [DESTROYED] = destroyed,
        [3] = obstacle.flinch,
        [4] = obstacle.paralyzed,
        [5] = obstacle.knocked_back,
        [WAITING] = wait,
        [SWINGING] = swinging,
    }

    -- 0x080D6974.
    local function tick(me: Object)
        obstacle.take_hits(me, "keeps_damage")
        local a = obstacle.react(me)
        if a then
            local action = ACTIONS[a]
            if not action then
                error(string.format("a hammer's action %d reads past 0x080D698C", a))
            end
            action(me)
        end
        me.collision:present()
    end

    -- 0x080D68F0: its look on its panel, 100 HP, its collision; its first tick at once.
    local function init(me: Object)
        me.visible = true
        local sprite = me.sprite
        sprite:load(SPRITE)
        sprite.shadow = "ground"
        me.anim, me.anim_loaded = look.stand_anim, 0xFF
        sprite:set_animation(look.stand_anim)
        sprite:step()
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        sprite.palette = look.palette
        me.hp, me.max_hp = HP, HP
        me:set_coordinates_from_panel()
        me.pos = Vec3.new(me.pos.x, me.pos.y, 0)
        me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
        if not me:create_collision() then
            me:free()
            return
        end
        me:setup_collision(collision.own_body, collision.hit_by_other_side, HIT_MOD)
        me.collision:present()
        me:set_lifecycle("update")
        tick(me)
    end

    local kind = new.kind {
        id = look.id,
        pool = "attack",
        -- 0x080D68CC: its state's routine, then its sprite.
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

    local function spawn(owner: Object, x: number, y: number, element: number, damage: number): Object?
        local o = battle.spawn(kind, Vec3.zero)
        if not o then
            return nil
        end
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.alliance = owner.alliance
        o:set_damage_word(damage)
        obstacle.register(o, o.alliance, 1)
        o.run_while_dimmed = true
        return o
    end

    return { kind = kind, spawn = spawn }
end

return hammer
