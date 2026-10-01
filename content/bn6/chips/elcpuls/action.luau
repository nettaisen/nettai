--!strict
-- ElcPuls's action (the original's 0x42, `sub_80EE55E`): the navi holds
-- out its coil and, a tick later, fires an electric pulse (attack object
-- #0x8C, ./pulse) from the panel ahead, holding it out for a while (the
-- pulse's ticks less 5, or 5: the original reads the pulse's look), then
-- puts the coil away over 7 ticks. Each chip composes its own (`action {
-- pulse = ..., hold = ... }`).

local attachment = require("../../objects/attachment/attachment")
local pulse = require("./pulse")

local action = {}

export type State = {
    -- Ticks left in the current phase (AIAttackVars+0x10), and the pulse's
    -- hold (AIAttackVars+0x12).
    timer: number,
    hold: number,
}

-- One state for every ElcPuls: a use continues the last one's attack
-- variables, as the original's one action does.
action.STATE = { timer = "u16", hold = "u16" }

-- The phases, on the attack's step.
local RAISE, PULSE, LOWER = 0x0, 0x4, 0x8

local ANIM_RAISE = 0x0A
-- The coil the navi holds (the original's attachment row 0x2B).
local COIL = attachment.look { sprite = asset.sprite("elec-coil"), palette = 0, attach_point = 0x0E }
local LOWER_TICKS = 6

-- The coil's animation steps (out, firing, away). Without a coil (its pool
-- was full) there is nothing to step.
local function step_coil(me: Object, by: number)
    local coil = me.overlay
    if coil then
        coil.anim += by
    end
end

-- A chip's ElcPuls: its pulse, and how long the navi holds it out.
function action.action(spec: ElecPulseSpec): Action

    local v = spec.pulse

    -- `sub_80EE57C`: the coil comes out.
    local function raise(me: Object, s: State)
        me:set_animation(ANIM_RAISE)
        me:open_counter_window()
        attachment.attach(me, COIL, "overlay")
        me:set_status("using_action", true)
        s.timer = 1
        me.step = PULSE
        me.step_init = 0
    end

    -- `sub_80EE5A8`: a tick later the pulse fires; hold it.
    local function fire(me: Object, s: State)
        if me.step_init == 0 then
            local t = s.timer - 1
            s.timer = t
            if t > 0 then
                return
            end
            step_coil(me, 1)
            s.hold = spec.hold
            me.step_init = 4
            local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
            pulse.spawn(me, me.panel_x + me.facing, me.panel_y, me.attack_element, v, damage)
        end
        local h = s.hold - 1
        s.hold = h
        if h > 0 then
            return
        end
        me.step = LOWER
        me.step_init = 0
    end

    -- `sub_80EE604`: put the coil away; 7 ticks later, back to idle. The
    -- coil and the pulse see their slots cleared.
    local function lower(me: Object, s: State)
        if me.step_init == 0 then
            step_coil(me, -1)
            s.timer = LOWER_TICKS
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
    local function update(me: Object, s: State)
        local step = me.step
        if step == RAISE then
            raise(me, s)
        elseif step == PULSE then
            fire(me, s)
        elseif step == LOWER then
            lower(me, s)
        else
            error(string.format("ElcPuls phase %#x reads past its phase table", step))
        end
    end

    return define.action { id = spec.id, state = action.STATE, args = spec, update = update }
end

return action
