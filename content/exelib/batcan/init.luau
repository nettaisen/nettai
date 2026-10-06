--!strict
-- BatCan1-4, as EXE5 and EXE6 share them (EXE6's action 0x19, `sub_80EC0E6`; EXE5's 0x36, 0x080EE410; EXE6's
-- docs/engine/shot-chips.md §6): the navi raises its bat cannon and fires an unseen shot from the panel ahead on the
-- 7th tick; BatCan4 fires three, 16 ticks apart, the last staggering harder. The shot (EXE6's attack object #1,
-- `sub_80C50B8`; EXE5's #1, 0x080C49FC) is the projectile's twin without a look: it steps a panel every two ticks
-- from where it appears and ends on its first hit, with the bat's spark; what differs between the shots is its hit
-- modifier. (Its record also gives a word its hits add into the target's collision data, `sub_80C518C`'s, which
-- nothing in either game reads; it isn't kept.)
--
-- Each game makes its own (`batcan.make`) with its look: the bat cannon, the sound, the shot's collision types and
-- spark, and whether raising the cannon restarts the navi's form overlay (EXE6's; EXE5's 0x080EE448 doesn't).

local attachment = require("@exelib/attachment")

local batcan = {}

export type Look = {
    -- The shot kind's id.
    shot_id: string,
    -- The bat cannon (EXE6's attachment row 0xF, EXE5's 0x21: it sits at a fixed offset by its holder), and the
    -- game's attachment kind that holds it.
    bat: AttachmentLook,
    attachment: Kind,
    sound: Sound,
    shot: { is: CollisionType, hits: CollisionType, spark: Spark },
    -- Raising the cannon restarts the navi's form overlay (EXE6's).
    restarts_overlay: boolean,
}

-- A chip's shots: each one's hit modifier, in the order they're fired (the chip parameters, or BatCan4's
-- `byte_80EC1BC`).
export type BatCanSpec = { shots: { number } }

export type BatCan = {
    shot: Kind,
    -- `sub_80C5176`: a shot from `owner` on panel (x, y).
    fire: (owner: Object, x: number, y: number, z: number, element: number, damage: number, hit_mod: number) -> Object?,
    -- A chip's action.
    action: (spec: BatCanSpec) -> Action,
}

type ShotState = { hit_mod: number }

export type State = {
    -- The phase's count (AIAttackVars+0x10).
    timer: number,
}

-- One state for the four (their shots left are the attack's word at AIAttackVars+0x12, the navi's `attack_count`).
local STATE = { timer = "u16" }

-- The phases, on the attack's step.
local COUNT, FIRE, RECOVER = 0x0, 0x4, 0x8

local ANIM_FIRE, ANIM_DONE = 9, 0
-- The shot leaves on this tick of the count, and the phase ends on this one; the recovery's ticks.
local SHOT_TICK, FIRE_TICKS, RECOVER_TICKS = 5, 0xF, 0x14
local SHOT_Z = 0x18_0000
-- Ticks the shot waits on a panel after the one it enters on (Timer2).
local WAIT = 1

function batcan.make(look: Look): BatCan
    -- Done: off its panels, and freed at the next update.
    local function finish(me: Object)
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
    end

    -- `sub_80C5116`: the hits against it resolve and spark; it ends with the battle or on a hit, else it waits out
    -- its panel and steps forward.
    local function tick(me: Object)
        local c = me.collision
        c:remove()
        me:hit_spark()
        if battle.over() or c.hit_flags ~= 0 then
            finish(me)
        else
            if me.phase_init == 0 then
                me.timer = me.timer2
                me.phase_init = 4
            end
            local left = me.timer - 1
            me.timer = left
            if left < 0 then
                me.panel_x = me.panel_x + me.facing
                if not field.valid(me.panel_x, me.panel_y) then
                    finish(me)
                else
                    me:set_coordinates_from_panel()
                    me:update_collision_panels()
                    me.phase_init = 0
                end
            end
        end
        c:present()
    end

    -- `sub_80C50DC`: on its panel, an attack of its side with its hit modifier and the bat's spark.
    local function init(me: Object)
        me:set_coordinates_from_panel()
        me.timer2 = WAIT
        if not me:create_collision() then
            me:free()
            return
        end
        me:setup_collision(look.shot.is, look.shot.hits, (me.state :: ShotState).hit_mod)
        me.collision:set_hit_effect(look.shot.spark)
        me.collision:present()
        me:set_lifecycle("update")
        tick(me)
    end

    local shot = new.kind {
        id = look.shot_id,
        pool = "attack",
        state = { hit_mod = "u8" },
        -- `sub_80C50B8`.
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

    -- `sub_80C5176`: a shot from `owner` on panel (x, y) at height `z` (16.16), with `element`, the damage word and
    -- the hit modifier; nil if the attack pool is full. It takes the owner's side and flip, runs later this tick,
    -- right after its spawner, and keeps its Z.
    local function fire(owner: Object, x: number, y: number, z: number, element: number, damage: number, hit_mod: number): Object?
        local o = battle.spawn(shot, Vec3.new(0, 0, z))
        if o then
            (o.state :: ShotState).hit_mod = hit_mod
            o.panel_x, o.panel_y = x, y
            o.element = element
            o:set_damage_word(damage)
            o.alliance, o.flip = owner.alliance, owner.flip
            o.related1 = owner
        end
        return o
    end

    local function action(spec: BatCanSpec): Action
        -- `sub_80EC104`: how many shots.
        local function count(me: Object)
            me.attack_count = #spec.shots
            me.step, me.step_init = FIRE, 0
        end

        -- `sub_80EC11C`: raise a new bat cannon (EXE6's form overlay restarts with the navi's animation); fire on the
        -- count's 5; at 0xF the next shot, or the recovery.
        local function fire_step(me: Object, s: State)
            if me.step_init == 0 then
                me:set_animation(ANIM_FIRE)
                if look.restarts_overlay then
                    local overlay = me.related2
                    if overlay then
                        overlay.anim_loaded = 0xFF
                    end
                end
                me:open_counter_window()
                s.timer = 0
                local held = me.related1
                if held then
                    held:set_lifecycle("destroy")
                end
                attachment.attach(look.attachment, me, look.bat, "related")
                me:set_status("using_action", true)
                me.step_init = 4
                return
            end
            if s.timer == SHOT_TICK then
                battle.play_sound(look.sound)
                local hit_mod = spec.shots[#spec.shots - me.attack_count + 1]
                local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                fire(me, me.panel_x + me.facing, me.panel_y, SHOT_Z, me.attack_element, damage, hit_mod)
            end
            local t = s.timer + 1
            s.timer = t
            if t == FIRE_TICKS then
                local left = me.attack_count - 1
                me.attack_count = left
                if left > 0 then
                    me.step_init = 0
                else
                    me.step, me.step_init = RECOVER, 0
                end
            end
        end

        -- `sub_80EC1C4`: 21 ticks, then the navi's plain animation, the cannon goes and back to idle.
        local function recover(me: Object, s: State)
            if me.step_init == 0 then
                s.timer = RECOVER_TICKS
                me.step_init = 4
            end
            local t = s.timer - 1
            s.timer = t
            if t < 0 then
                me:set_animation(ANIM_DONE)
                me.related1 = nil
                me.overlay = nil
                me:exit_attack()
            end
        end

        return new.action {
            state = STATE,
            args = spec,
            -- `sub_80EC0E6`.
            update = function(me: Object, s: State)
                local step = me.step
                if step == COUNT then
                    count(me)
                elseif step == FIRE then
                    fire_step(me, s)
                elseif step == RECOVER then
                    recover(me, s)
                else
                    error(string.format("BatCan phase %#x reads past off_80EC0F8", step))
                end
            end,
        }
    end

    return { shot = shot, fire = fire, action = action }
end

return batcan
