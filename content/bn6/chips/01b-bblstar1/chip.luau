--!strict
-- BblStar1, BblStar2 and BblStar3: action 0x2D (`sub_80ED2F8`). The navi
-- blows a bubble star (attack object #0x42, objects/bubble-star) from the
-- panel ahead on the third tick; it drifts forward along a wave and traps
-- what it hits in a bubble. The chip's first parameter picks the star's
-- speed and palette. 32 ticks in all.

local bubble_star = require("../../objects/bubble-star/bubble_star")

export type State = {
    -- Ticks left in the current phase (AIAttackVars+0x10).
    timer: number,
}

local bbl_star = {
    state = { timer = "u16" },
}

-- The phases, on the attack's step.
local BLOW, RECOVER = 0x0, 0x4
-- The tick (timer value) the star is blown.
local BLOW_AT = 0x0D

-- `sub_80ED314`: blow the star on the timer's 0xD, then recover.
local function blow(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0x0C)
        battle.play_sound(0xD8)
        me:open_counter_window()
        me:set_status("using_action", true)
        s.timer = 0x0F
        me.step_init = 4
    end
    if s.timer == BLOW_AT then
        -- The damage word (the hit parameter in its high half), plus the
        -- Atk+ bonus.
        local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
        local params = { me:attack_param(1), me:attack_param(2), me:attack_param(3), me:attack_param(4) }
        bubble_star.spawn(me, me.panel_x + me.facing, me.panel_y, me.attack_element, damage, params)
        battle.play_sound(0xB4)
    end
    local t = s.timer - 1
    s.timer = t
    if t < 0 then
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

-- `sub_80ED2F8`.
function bbl_star.update(me: Object, s: State)
    local step = me.step
    if step == BLOW then
        blow(me, s)
    elseif step == RECOVER then
        recover(me, s)
    else
        error(string.format("BblStar phase %#x reads past its phase table", step))
    end
end

return bbl_star
