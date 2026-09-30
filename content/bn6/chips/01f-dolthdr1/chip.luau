--!strict
-- DolThdr1/2/3: action 0x3E (`sub_80EE0BC`). The navi holds out its doll
-- (attachment 0x29; the buster arm, animation 0x19, when the chip's third
-- parameter is set), and 10 ticks later the doll (attack object #0x82,
-- objects/thunder-doll) strikes thunder along the row ahead, held for 15
-- ticks; then the navi puts it away over 7 ticks.

local attachment = require("../../objects/attachment/attachment")
local thunder_doll = require("../../objects/thunder-doll/thunder_doll")

export type State = {
    -- Ticks left in the current phase (AIAttackVars+0x10), and the
    -- thunder's hold (AIAttackVars+0x12).
    timer: number,
    hold: number,
}

local dol_thdr = {
    state = { timer = "u16", hold = "u16" },
}

-- The phases, on the attack's step.
local RAISE, STRIKE, LOWER = 0x0, 0x4, 0x8
-- What the navi holds: the doll, or (third parameter set) the buster arm
-- (`dword_80EE29C`: attachment 6, animation 0x19).
local DOLL = { kind = 0x29 }
local ARM = { kind = 6, anim = 0x19 }
local HOLD = 0x0F

-- The held attachment's animation steps. Without one (its pool was full)
-- there is nothing to step.
local function step_held(me: Object, by: number)
    local held = me.overlay
    if held then
        held.anim += by
    end
end

-- `sub_80EE0DC`: the doll comes out.
local function raise(me: Object, s: State)
    me:set_animation(0x0A)
    me:open_counter_window()
    attachment.spawn(me, if me:attack_param(3) ~= 0 then ARM else DOLL, "overlay")
    me:set_status("using_action", true)
    s.timer = 0x0A
    me.step = STRIKE
    me.step_init = 0
end

-- `sub_80EE110`: 10 ticks later the doll strikes; hold it.
local function strike(me: Object, s: State)
    if me.step_init == 0 then
        local t = s.timer - 1
        s.timer = t
        if t > 0 then
            return
        end
        step_held(me, 1)
        s.hold = HOLD
        me.step_init = 4
        local params = { me:attack_param(1), me:attack_param(2), me:attack_param(3), me:attack_param(4) }
        local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
        -- (The game hands the doll the navi's attack variables as its
        -- slot; DolThdr's first parameter is 0, so it never registers.)
        thunder_doll.spawn(me, me.panel_x + me.facing, me.panel_y, me.attack_element, params, damage, nil)
        battle.play_sound(0x19A)
    end
    local h = s.hold - 1
    s.hold = h
    if h > 0 then
        return
    end
    me.step = LOWER
    me.step_init = 0
end

-- `sub_80EE162`: put it away; 7 ticks later, back to idle.
local function lower(me: Object, s: State)
    if me.step_init == 0 then
        step_held(me, -1)
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

-- `sub_80EE0BC`.
function dol_thdr.update(me: Object, s: State)
    local step = me.step
    if step == RAISE then
        raise(me, s)
    elseif step == STRIKE then
        strike(me, s)
    elseif step == LOWER then
        lower(me, s)
    else
        error(string.format("DolThdr phase %#x reads past its phase table", step))
    end
end

return dol_thdr
