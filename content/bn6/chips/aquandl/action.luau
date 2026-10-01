--!strict
-- AquaNdl's action (the original's 0x32, `sub_80ED6E6`): the navi raises
-- its arm and launches a volley (effect object #0x40, ./volley) that drops
-- three aqua needles on the enemy's panels, 8 ticks apart, in the chip's
-- palette. 32 ticks in all. Each chip composes its own
-- (`action { palette = ... }`: the original's first parameter).

local volley = require("./volley")

local action = {}

export type State = {
    -- Ticks left in the current phase (AIAttackVars+0x10).
    timer: number,
}

-- One state for every AquaNdl: a use continues the last one's attack
-- variables, as the original's one action does.
action.STATE = { timer = "u16" }

-- The phases, on the attack's step.
local LAUNCH, RECOVER = 0x0, 0x4

local ANIM_RAISE = 0x0C
-- The phase's ticks, and the tick (timer value) the volley starts.
local LAUNCH_TICKS, LAUNCH_AT = 0x0F, 0x0D

-- A chip's AquaNdl: its needles' palette.
function action.action(spec: AquaNeedleSpec): Action
    -- `sub_80ED700`: raise the arm; launch on the timer's 0xD.
    local function launch(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(ANIM_RAISE)
            me:open_counter_window()
            me:set_status("using_action", true)
            s.timer = LAUNCH_TICKS
            me.step_init = 4
        end
        if s.timer == LAUNCH_AT then
            local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
            volley.spawn(me, me.attack_element, spec.palette, damage)
        end
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            s.timer = LAUNCH_TICKS
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
    local function update(me: Object, s: State)
        local step = me.step
        if step == LAUNCH then
            launch(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("AquaNdl phase %#x reads past its phase table", step))
        end
    end

    return define.action { id = spec.id, state = action.STATE, args = spec, update = update }
end

return action
