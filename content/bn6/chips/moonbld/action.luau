--!strict
-- MoonBld's action (the original's 0x40, `sub_80EE2A0`): the navi spins
-- with a blade (attack object #0x85, ./blade) that sweeps the eight panels
-- around it, from the fourth tick. The swing lasts 22 ticks and repeats
-- while a count says so (MoonBld's count is 1: one swing); then back to
-- idle.

local blade = require("./blade")

local action = {}

export type State = {
    -- Ticks left in the swing (AIAttackVars+0x10). (The swings left are
    -- the navi's `attack_count`.)
    timer: number,
}

-- One state for every MoonBld action.
action.STATE = { timer = "u16" }

-- The phases, on the attack's step.
local READY, SWING, DONE = 0x0, 0x4, 0x8

local ANIM_SPIN = 0x0C
local SOUND = asset.sound("spin")
local SWINGS = 1
-- The phase's ticks, and the tick (timer value) the blade comes out.
local SWING_TICKS, BLADE_AT = 0x15, 0x12
-- The low byte of the swing routine's address (`sub_80EE2CE`), which the
-- blade's spawner leaves in its panel Y.
local SWING_ROUTINE_LOW = 0xCF

-- MoonBld's swing.
function action.action(spec: { id: string?, charged: ChargedSlash? }): Action
    -- `sub_80EE2C0`: one swing; the marker cleared.
    local function ready(me: Object, s: State)
        me.attack_count = SWINGS
        me.marker = 0
        me.step = SWING
        me.step_init = 0
    end

    -- `sub_80EE2CE`: the swing; the blade on the timer's 0x12; again while
    -- swings are left, else done.
    local function swing(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(ANIM_SPIN)
            me:open_counter_window()
            battle.play_sound(SOUND)
            me:set_status("using_action", true)
            s.timer = SWING_TICKS
            me.step_init = 4
        end
        if s.timer == BLADE_AT then
            -- The damage word (the hit parameter in its high half), plus
            -- the Atk+ bonus.
            local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
            blade.spawn(me, bit32.band(me.extra, 0xFF), SWING_ROUTINE_LOW, damage, false)
        end
        local t = s.timer - 1
        s.timer = t
        if t >= 0 then
            return
        end
        local left = me.attack_count - 1
        me.attack_count = left
        if left > 0 then
            me.related1 = nil
            me.overlay = nil
            me.step_init = 0
            return
        end
        me.step = DONE
        me.step_init = 0
    end

    -- `sub_80EE33A`: back to idle.
    local function done(me: Object)
        me.related1 = nil
        me.overlay = nil
        me:exit_attack()
    end

    -- `sub_80EE2A0`.
    local function update(me: Object, s: State)
        local step = me.step
        if step == READY then
            ready(me, s)
        elseif step == SWING then
            swing(me, s)
        elseif step == DONE then
            done(me)
        else
            error(string.format("MoonBld phase %#x reads past its phase table", step))
        end
    end

    return define.action { id = spec.id, state = action.STATE, args = spec, update = update }
end

return action
