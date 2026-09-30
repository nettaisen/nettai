--!strict
-- AquaNdl1/2/3: action 0x32 (`sub_80ED6E6`). The navi raises its arm and
-- launches a volley (effect object #0x40, objects/needle-volley) that
-- drops three aqua needles on the enemy's panels, 8 ticks apart; the
-- chip's first parameter tints the needles. 32 ticks in all.

local needle_volley = require("../../objects/needle-volley/needle_volley")

export type State = {
    -- Ticks left in the current phase (AIAttackVars+0x10).
    timer: number,
}

local aqua_needle = {
    state = { timer = "u16" },
}

-- The phases, on the attack's step.
local LAUNCH, RECOVER = 0x0, 0x4
-- The tick (timer value) the volley starts.
local LAUNCH_AT = 0x0D

-- `sub_80ED700`: raise the arm; launch on the timer's 0xD.
local function launch(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0x0C)
        me:open_counter_window()
        me:set_status("using_action", true)
        s.timer = 0x0F
        me.step_init = 4
    end
    if s.timer == LAUNCH_AT then
        local params = { me:attack_param(1), me:attack_param(2), me:attack_param(3), me:attack_param(4) }
        local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
        needle_volley.spawn(me, me.attack_element, params, damage)
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

-- `sub_80ED6E6`.
function aqua_needle.update(me: Object, s: State)
    local step = me.step
    if step == LAUNCH then
        launch(me, s)
    elseif step == RECOVER then
        recover(me, s)
    else
        error(string.format("AquaNdl phase %#x reads past its phase table", step))
    end
end

return aqua_needle
