--!strict
-- AirHocky and PitHocky: action 0x26 (`sub_80ECCB0`). The navi flicks a
-- puck (attack object #0x2E, objects/hockey-puck) from the panel ahead on
-- the third tick, slanting down (second parameter 1) or up; it bounces
-- around the field, a panel per 6 ticks. The chip's first parameter picks
-- how many steps it lasts (PitHocky's leaves a trail). 31 ticks in all.

local hockey_puck = require("../../objects/hockey-puck/hockey_puck")

export type State = {
    -- Ticks left in the current phase (AIAttackVars+0x10).
    timer: number,
}

local air_hocky = {
    state = { timer = "u16" },
}

-- The phases, on the attack's step.
local FLICK, RECOVER = 0x0, 0x4
-- The tick (timer value) the puck goes.
local FLICK_AT = 0x0D
-- The puck's height at the spawn (the registers; its init lands it).
local HEIGHT = 0x0A_0000

-- `sub_80ECCCC`: flick the puck on the timer's 0xD, then recover.
local function flick(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0x0C)
        me:open_counter_window()
        me:set_status("using_action", true)
        me.step_init = 4
        s.timer = 0x0F
    end
    if s.timer == FLICK_AT then
        -- The damage word (the hit parameter in its high half), plus the
        -- Atk+ bonus.
        local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
        local params = { me:attack_param(1), me:attack_param(2), me:attack_param(3), me:attack_param(4) }
        hockey_puck.spawn(me, me.panel_x + me.facing, me.panel_y, me.attack_element, HEIGHT, damage, params)
    end
    local t = s.timer - 1
    s.timer = t
    if t <= 0 then
        s.timer = 0x0F
        me.step = RECOVER
        me.step_init = 0
    end
end

-- `sub_80ECA0C`: 16 ticks, then back to idle.
local function recover(me: Object, s: State)
    local t = s.timer - 1
    s.timer = t
    if t < 0 then
        me:exit_attack()
    end
end

-- `sub_80ECCB0`.
function air_hocky.update(me: Object, s: State)
    local step = me.step
    if step == FLICK then
        flick(me, s)
    elseif step == RECOVER then
        recover(me, s)
    else
        error(string.format("AirHocky phase %#x reads past its phase table", step))
    end
end

return air_hocky
