--!strict
-- RlngLog's action (the original's 0x36, `sub_80ED9AE`): the navi takes
-- out the log thrower and throws two logs (attack object #0x66, ./log)
-- onto the panel ahead, 13 ticks apart: from the top row both on row 2,
-- from the middle row on rows 2 and 3, from the bottom row both on row 3
-- (each big log also covers the row above it). Each throw raises the
-- thrower (its animation 1) 3 ticks before; after the second the navi
-- recovers for 5 ticks. Each chip composes its own (`action { log = ...
-- }`: the log, the original's parameters).

local attachment = require("../../objects/attachment")
local log = require("./log")

local action = {}

export type State = {
    -- Ticks left in the current phase (AIAttackVars+0x10). (The logs left
    -- to throw are the navi's `attack_count`.)
    timer: number,
}

-- One state for every RlngLog: a use continues the last one's attack
-- variables, as the original's one action does.
action.STATE = { timer = "u16" }

-- The phases, on the attack's step.
local READY, THROW, RECOVER = 0x0, 0x4, 0x8
-- The attack's step_init values in THROW: winding up, waiting after a
-- throw.
local WIND_UP, WAIT = 4, 8

local ANIM_THROW = 0x0A
-- The log thrower (the original's attachment row 0x27).
local THROWER = attachment.look { sprite = asset.sprite("log-thrower"), palette = 0, attach_point = 0x0E }
-- The row each log lands on, by the navi's row and the logs left
-- (`byte_80EDAB4`, at (row - 1) * 2 + logs - 1).
local ROWS = { [0] = 2, 2, 3, 2, 3, 3 }
local SOUND_THROW, SOUND_ROLL = asset.sound("throw"), asset.sound("rlnglog1-roll")

-- A chip's RlngLog: the logs it throws.
function action.action(spec: RollingLogSpec): Action
    -- `sub_80ED9CC`: the thrower out; two ticks later, the throws.
    local function ready(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(ANIM_THROW)
            me:open_counter_window()
            attachment.attach(me, THROWER, "overlay")
            me:set_status("using_action", true)
            me.step_init = 4
            s.timer = 1
        end
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            me.attack_count = 2
            me.step = THROW
            me.step_init = 0
        end
    end

    -- `sub_80EDA0E`: raise the thrower, 4 ticks later throw a log, wait
    -- 10; again for the second log, then recover.
    local function throw(me: Object, s: State)
        local init = me.step_init
        if init == WAIT then
            local t = s.timer - 1
            s.timer = t
            if t <= 0 then
                me.step_init = 0
            end
            return
        end
        if init ~= WIND_UP then
            s.timer = 3
            local thrower = me.overlay
            if thrower then
                thrower:set_animation(1)
            end
            me.step_init = WIND_UP
        end
        local t = s.timer - 1
        s.timer = t
        if t >= 0 then
            return
        end
        s.timer = 0x0A
        local i = (me.panel_y - 1) * 2 + me.attack_count - 1
        local row = ROWS[i]
        if row == nil then
            error(string.format("RlngLog reads byte_80EDAB4 at %d, past its end", i))
        end
        -- The damage word (the hit parameter in its high half), plus the
        -- Atk+ bonus.
        local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
        log.spawn(me, me.panel_x + me.facing, row, me.attack_element, spec.log, damage)
        battle.play_sound(SOUND_THROW)
        battle.play_sound(SOUND_ROLL)
        me.step_init = WAIT
        s.timer = 0x0A
        local left = me.attack_count - 1
        me.attack_count = left
        if left <= 0 then
            me.step = RECOVER
            me.step_init = 0
        end
    end

    -- `sub_80EDABA`: 5 ticks, then the thrower goes and back to idle.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            s.timer = 4
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

    -- `sub_80ED9AE`.
    local function update(me: Object, s: State)
        local step = me.step
        if step == READY then
            ready(me, s)
        elseif step == THROW then
            throw(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("RlngLog phase %#x reads past its phase table", step))
        end
    end

    return new.action { id = spec.id, state = action.STATE, args = spec, update = update }
end

return action
