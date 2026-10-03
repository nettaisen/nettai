--!strict
-- WaveArm's and PwrWave's strike (the original's action 0x31,
-- `sub_80ED64C`): the navi strikes the ground and sends a shock wave
-- (wave.luau) of its chip's variant rolling from the panel ahead; PwrWave
-- also sends one along the rows above and below from its own column. 32
-- ticks in all.

local wave = require("./wave")

local strike = {}

export type State = {
    -- Ticks left in the current phase (AIAttackVars+0x10).
    timer: number,
}

-- A chip's strike: the wave it sends, and (PwrWave) the two more from its
-- own column; `id`, the action's key when no chip definition holds it.
export type Spec = { id: string?, wave: ShockWaveVariant, three_rows: boolean? }

-- One state for every strike.
strike.STATE = { timer = "u16" }

-- The phases, on the attack's step.
local STRIKE, RECOVER = 0x0, 0x4
-- The tick (timer value) the waves go.
local SEND_AT = 0x0D
local SOUND = asset.sound("bn6:bblstar1")

function strike.action(spec: Spec): Action
    -- `sub_80ED6CA`: a wave from (dx, dy) off the navi's panel, with the
    -- chip's damage.
    local function send(me: Object, dx: number, dy: number)
        local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
        wave.spawn(me, me.panel_x + dx, me.panel_y + dy, spec.wave, damage)
    end

    -- `sub_80ED668`: strike; the waves go on the timer's 0xD.
    local function striking(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(0x0C)
            battle.play_sound(SOUND)
            me:open_counter_window()
            me:set_status("using_action", true)
            s.timer = 0x0F
            me.step_init = 4
        end
        if s.timer == SEND_AT then
            send(me, me.facing, 0)
            if spec.three_rows then
                send(me, 0, -1)
                send(me, 0, 1)
            end
        end
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            s.timer = 0x0F
            me.step = RECOVER
            me.step_init = 0
        end
    end

    -- `sub_80ECA0C` (CrakShot's too): 16 ticks, then back to idle.
    local function recover(me: Object, s: State)
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            me:exit_attack()
        end
    end

    -- `sub_80ED64C`.
    local function update(me: Object, s: State)
        local step = me.step
        if step == STRIKE then
            striking(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("WaveArm phase %#x reads past its phase table", step))
        end
    end

    return define.action { id = spec.id, state = strike.STATE, args = spec, update = update }
end

return strike
