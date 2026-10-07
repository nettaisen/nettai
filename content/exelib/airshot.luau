--!strict
-- AirShot's action, as EXE6, EXE5 and EXE4 share it (EXE6's action 0x21,
-- `sub_80EC884`; EXE5's 0x1D, the same code but for the shooter's
-- attachment row; EXE4's 0x23, 0x080ECB74, emap: its fire and recovery
-- EXE5's, similar; docs/engine/shot-chips.md §3): a shot from the panel
-- ahead. The navi raises its air shooter and fires on the 7th tick; the
-- recovery after the 11th. A chip gives what the navi holds, what it fires
-- and its recovery's count (`airshot.action({ shooter = ..., shot = ...,
-- recovery = ... }, look)`, with the game's look: the sound, how the navi
-- holds the shooter, the projectiles, the navi's animation and the
-- shooter's slot, EXE4's own in each but the sound and projectiles).

local airshot = {}

export type State = {
    -- Ticks since the shooter came up, then the recovery's countdown.
    timer: number,
}

local STATE = { timer = "u16" }

-- The phases, on the attack's step.
local FIRE, RECOVER = 0x0, 0x4

-- (A game's AirShot is an `AirShotLook`: its sound, how the navi holds the
-- shooter (EXE6's `sub_80EC934`: by the holder's AI index, from
-- `off_80EC944`, where the link navis HeatMan, GroundMan and DustMan hold
-- their own, 1; unlike the cannon's it doesn't ask whether the holder is a
-- player), its projectiles, the navi's animation and the shooter's slot.)
type Look = AirShotLook
-- The shot leaves on this tick of the count, and the phase ends on this
-- one.
local SHOT_TICK, FIRE_TICKS = 5, 0xA
local SHOT_Z = 0x18_0000

-- An AirShot: what the navi holds and what it fires, of a game's `look`.
function airshot.action(spec: AirShotSpec, look: Look): Action
    local SOUND = look.sound
    local projectile = look.projectiles
    -- `sub_80EC8A0`: raise the shooter (and its sound); fire on the count's 5;
    -- the next phase at 0xA.
    local function fire(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(look.anim)
            me:open_counter_window()
            s.timer = 0
            me:set_status("using_action", true)
            look.hold(me, spec.shooter, look.slot)
            battle.play_sound(SOUND)
            me.step_init = 4
            return
        end
        if s.timer == SHOT_TICK then
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
            projectile.spawn(me, me.panel_x + me.facing, me.panel_y, { variant = spec.shot, damage = damage, z = SHOT_Z })
        end
        local t = s.timer + 1
        s.timer = t
        if t == FIRE_TICKS then
            me.step, me.step_init = RECOVER, 0
        end
    end

    -- `sub_80EC90E`: the recovery's count and a tick, then the shooter goes
    -- and back to idle.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            s.timer = spec.recovery
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            me.related1 = nil
            me.overlay = nil
            me:exit_attack()
        end
    end

    return new.action {
        id = spec.id,
        state = STATE,
        -- `sub_80EC884`.
        update = function(me: Object, s: State)
            local step = me.step
            if step == FIRE then
                fire(me, s)
            elseif step == RECOVER then
                recover(me, s)
            else
                error(string.format("AirShot phase %#x reads past off_80EC898", step))
            end
        end,
    }
end

return airshot
