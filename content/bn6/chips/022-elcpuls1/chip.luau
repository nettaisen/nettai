--!strict
-- ElcPuls1/2/3 and DestPuls: action 0x42 (`sub_80EE55E`). The navi holds
-- out its coil (attachment 0x2B) and, a tick later, fires an electric
-- pulse (attack object #0x8C, objects/elec-pulse) from the panel ahead,
-- holding it for the chip's third parameter less 5 ticks (5 when its
-- fourth is 1), then puts the coil away over 7 ticks. The chip's
-- parameters are the pulse's: hit modifier, status, ticks, look.

local attachment = require("../../objects/attachment/attachment")
local elec_pulse = require("../../objects/elec-pulse/elec_pulse")

export type State = {
    -- Ticks left in the current phase (AIAttackVars+0x10), and the pulse's
    -- hold (AIAttackVars+0x12).
    timer: number,
    hold: number,
}

local elc_puls = {
    state = { timer = "u16", hold = "u16" },
}

-- The phases, on the attack's step.
local RAISE, PULSE, LOWER = 0x0, 0x4, 0x8
-- The coil the navi holds.
local COIL = 0x2B

-- The coil's animation steps (out, firing, away). Without a coil (its pool
-- was full) there is nothing to step.
local function step_coil(me: Object, by: number)
    local coil = me.overlay
    if coil then
        coil.anim += by
    end
end

-- `sub_80EE57C`: the coil comes out.
local function raise(me: Object, s: State)
    me:set_animation(0x0A)
    me:open_counter_window()
    attachment.spawn(me, { kind = COIL }, "overlay")
    me:set_status("using_action", true)
    s.timer = 1
    me.step = PULSE
    me.step_init = 0
end

-- `sub_80EE5A8`: a tick later the pulse fires; hold it.
local function pulse(me: Object, s: State)
    if me.step_init == 0 then
        local t = s.timer - 1
        s.timer = t
        if t > 0 then
            return
        end
        step_coil(me, 1)
        s.hold = if me:attack_param(4) == 1 then 5 else me:attack_param(3) - 5
        me.step_init = 4
        local params = { me:attack_param(1), me:attack_param(2), me:attack_param(3), me:attack_param(4) }
        local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
        elec_pulse.spawn(me, me.panel_x + me.facing, me.panel_y, me.attack_element, params, damage)
    end
    local h = s.hold - 1
    s.hold = h
    if h > 0 then
        return
    end
    me.step = LOWER
    me.step_init = 0
end

-- `sub_80EE604`: put the coil away; 7 ticks later, back to idle. The coil
-- and the pulse see their slots cleared.
local function lower(me: Object, s: State)
    if me.step_init == 0 then
        step_coil(me, -1)
        s.timer = 6
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = t
    if t >= 0 then
        return
    end
    me.related1 = nil
    me.overlay = nil
    me:exit_attack()
end

-- `sub_80EE55E`.
function elc_puls.update(me: Object, s: State)
    local step = me.step
    if step == RAISE then
        raise(me, s)
    elseif step == PULSE then
        pulse(me, s)
    elseif step == LOWER then
        lower(me, s)
    else
        error(string.format("ElcPuls phase %#x reads past its phase table", step))
    end
end

return elc_puls
