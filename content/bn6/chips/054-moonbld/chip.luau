--!strict
-- MoonBld: action 0x40 (`sub_80EE2A0`). The navi spins with a blade (attack
-- object #0x85, objects/moon-blade) that sweeps the eight panels around
-- it, from the fourth tick. The swing lasts 22 ticks and repeats while a
-- count says so (MoonBld's count is 1: one swing); then back to idle.

local moon_blade = require("../../objects/moon-blade/moon_blade")

export type State = {
    -- Ticks left in the swing (AIAttackVars+0x10).
    timer: number,
    -- Swings left (AIAttackVars+0x12).
    swings: number,
}

local moon_bld = {
    state = { timer = "u16", swings = "u16" },
}

-- The phases, on the attack's step.
local READY, SWING, DONE = 0x0, 0x4, 0x8

-- The tick (timer value) the blade comes out.
local BLADE_AT = 0x12
-- The low byte of the swing routine's address (`sub_80EE2CE`), which the
-- blade's spawner leaves in its panel Y.
local SWING_ROUTINE_LOW = 0xCF

-- `sub_80EE2C0`: one swing; the marker cleared.
local function ready(me: Object, s: State)
    s.swings = 1
    me.marker = 0
    me.step = SWING
    me.step_init = 0
end

-- `sub_80EE2CE`: the swing; the blade on the timer's 0x12; again while
-- swings are left, else done.
local function swing(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0x0C)
        me:open_counter_window()
        battle.play_sound(0xC7)
        me:set_status("using_action", true)
        s.timer = 0x15
        me.step_init = 4
    end
    if s.timer == BLADE_AT then
        -- The damage word (the hit parameter in its high half), plus the
        -- Atk+ bonus.
        local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
        moon_blade.spawn(me, bit32.band(me.extra, 0xFF), SWING_ROUTINE_LOW, damage, { 0, 0, 0, 0 })
    end
    local t = s.timer - 1
    s.timer = t
    if t >= 0 then
        return
    end
    local left = s.swings - 1
    s.swings = left
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
function moon_bld.update(me: Object, s: State)
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

return moon_bld
