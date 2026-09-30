--!strict
-- MagCoil: action 0x44 (`sub_80EE6FC`). The navi holds out its coil
-- (attachment 0x2C) and, a tick later, sets a magnet (attack object #0x95,
-- objects/magnet) on the panel ahead that pulls the enemies in the rows
-- beside it toward its row for the chip's first parameter's ticks; the
-- navi holds 10 ticks, then puts the coil away over 7.

local attachment = require("../../objects/attachment/attachment")
local magnet = require("../../objects/magnet/magnet")

export type State = {
    -- Ticks left in the current phase (AIAttackVars+0x10), and the hold
    -- (AIAttackVars+0x12).
    timer: number,
    hold: number,
}

local mag_coil = {
    state = { timer = "u16", hold = "u16" },
}

-- The phases, on the attack's step.
local RAISE, SET, LOWER = 0x0, 0x4, 0x8
local COIL = 0x2C
local HOLD = 0x0A

-- The coil's animation steps. Without a coil (its pool was full) there is
-- nothing to step.
local function step_coil(me: Object, by: number)
    local coil = me.overlay
    if coil then
        coil.anim += by
    end
end

-- `sub_80EE71C`: the coil comes out.
local function raise(me: Object, s: State)
    me:set_animation(0x0A)
    me:open_counter_window()
    attachment.spawn(me, { kind = COIL }, "overlay")
    me:set_status("using_action", true)
    s.timer = 1
    me.step = SET
    me.step_init = 0
end

-- `sub_80EE748`: a tick later the magnet goes out; hold.
local function set(me: Object, s: State)
    if me.step_init == 0 then
        local t = s.timer - 1
        s.timer = t
        if t > 0 then
            return
        end
        step_coil(me, 1)
        s.hold = HOLD
        me.step_init = 4
        local params = { me:attack_param(1), me:attack_param(2), me:attack_param(3), me:attack_param(4) }
        magnet.spawn(me, me.panel_x + me.facing, me.panel_y, me.attack_element, params)
        battle.play_sound(0xF3)
    end
    local h = s.hold - 1
    s.hold = h
    if h > 0 then
        return
    end
    me.step = LOWER
    me.step_init = 0
end

-- `sub_80EE7A0`: put the coil away; 7 ticks later, back to idle.
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

-- `sub_80EE6FC`.
function mag_coil.update(me: Object, s: State)
    local step = me.step
    if step == RAISE then
        raise(me, s)
    elseif step == SET then
        set(me, s)
    elseif step == LOWER then
        lower(me, s)
    else
        error(string.format("MagCoil phase %#x reads past its phase table", step))
    end
end

return mag_coil
