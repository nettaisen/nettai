--!strict
-- AirShot's action, as EXE5 and EXE6 share it (EXE6's action 0x21, `sub_80EC884`;
-- EXE5's 0x1D, the same code but for the shooter's attachment row;
-- docs/engine/shot-chips.md §3): a shot from the panel ahead. The navi
-- raises its air shooter and fires on the 7th tick; 11 ticks of recovery
-- after the 11th. A chip gives what the navi holds and what it fires
-- (`airshot.action({ shooter = ..., shot = ... }, look)`, with the game's
-- sound, attachment and projectiles): EXE6's AirShot its shooter and wind
-- shot, EXE5's its own.

local attachment = require("./attachment")

local airshot = {}

export type State = {
    -- Ticks since the shooter came up, then the recovery's countdown.
    timer: number,
}

local STATE = { timer = "u16" }

-- The phases, on the attack's step.
local FIRE, RECOVER = 0x0, 0x4

local ANIM = 9

-- (A game's AirShot is a `ShooterLook`: its sound, attachment kind, the
-- shooter's, and projectiles.)
type Look = ShooterLook
-- The shot leaves on this tick of the count, and the phase ends on this
-- one; the recovery's ticks.
local SHOT_TICK, FIRE_TICKS, RECOVER_TICKS = 5, 0xA, 0xA
local SHOT_Z = 0x18_0000

-- `sub_80EC934`: the shooter's animation by the holder's AI index (the
-- link navis HeatMan, GroundMan and DustMan, 1, 9 and 10: 1; the others
-- 0). Unlike the cannon's it doesn't ask whether the holder is a player.
local function arm_anim(me: Object): number
    local ai = me.ai_index
    if ai == 1 or ai == 9 or ai == 10 then
        return 1
    end
    if ai > 27 then
        error(string.format("AI index %d reads past off_80EC944", ai))
    end
    return 0
end

-- An AirShot: what the navi holds and what it fires, of a game's `look`.
function airshot.action(spec: AirShotSpec, look: Look): Action
    local SOUND = look.sound
    local projectile = look.projectiles
    -- `sub_80EC8A0`: raise the shooter (and its sound); fire on the count's 5;
    -- the next phase at 0xA.
    local function fire(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(ANIM)
            me:open_counter_window()
            s.timer = 0
            me:set_status("using_action", true)
            attachment.attach(look.attachment, me, spec.shooter, "overlay", { anim = arm_anim(me) })
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

    -- `sub_80EC90E`: 11 ticks, then the shooter goes and back to idle.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            s.timer = RECOVER_TICKS
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

    return define.action {
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
