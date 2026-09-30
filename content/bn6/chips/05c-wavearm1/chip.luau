--!strict
-- WaveArm1/2/3 and PwrWave1/2/3: action 0x31 (`sub_80ED64C`). The navi
-- strikes the ground and sends a shock wave (attack object #0x16,
-- objects/shock-wave) rolling from the panel ahead; the chip's first
-- parameter is the wave's variant (WaveArm's steer toward an enemy's row,
-- PwrWave's roll straight). PwrWave (variant 1) also sends one along the
-- rows above and below from its own column. 32 ticks in all.

local shock_wave = require("../../objects/shock-wave/shock_wave")

export type State = {
    -- Ticks left in the current phase (AIAttackVars+0x10).
    timer: number,
}

local wave_arm = {
    state = { timer = "u16" },
}

-- The phases, on the attack's step.
local STRIKE, RECOVER = 0x0, 0x4
-- The tick (timer value) the waves go.
local SEND_AT = 0x0D
-- PwrWave's variant.
local POWER = 1

-- `sub_80ED6CA`: a wave from (dx, dy) off the navi's panel, with the
-- chip's parameters and damage.
local function send(me: Object, dx: number, dy: number)
    local params = { me:attack_param(1), me:attack_param(2), me:attack_param(3), me:attack_param(4) }
    local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
    shock_wave.spawn(me, me.panel_x + dx, me.panel_y + dy, params, damage)
end

-- `sub_80ED668`: strike; the waves go on the timer's 0xD.
local function strike(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0x0C)
        battle.play_sound(0xD8)
        me:open_counter_window()
        me:set_status("using_action", true)
        s.timer = 0x0F
        me.step_init = 4
    end
    if s.timer == SEND_AT then
        send(me, me.facing, 0)
        if me.variant == POWER then
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
function wave_arm.update(me: Object, s: State)
    local step = me.step
    if step == STRIKE then
        strike(me, s)
    elseif step == RECOVER then
        recover(me, s)
    else
        error(string.format("WaveArm phase %#x reads past its phase table", step))
    end
end

return wave_arm
