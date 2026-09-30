--!strict
-- TenguCross Beast's charged chip: weapon routine 0xA (`sub_8011C18`) and
-- action 0x3D (`sub_80EE044`). A charged Null-family chip becomes a
-- whirlwind (objects/whirlwind) from the panel in front when the navi's
-- wind-up animation ends; the navi holds for 46 ticks.

local weapon = require("../../../../lib/weapon")
local whirlwind = require("../../../../objects/whirlwind/whirlwind")

export type State = {
    -- Ticks left (AIAttackVars+0x10).
    timer: number,
}

local tengu_beast_charge = {
    state = { timer = "u16" },
}

local ACTION = 0x3D

-- The phases, on the attack's step.
local WIND_UP, HOLD = 0x0, 0x4

-- Wind (the attack's element: secondary element 0x20) and the hits'
-- counter byte.
local WIND, HIT_PARAM = 0x20, 0x8A

-- The whirlwind's parameters, the attack's (`dword_8011C34`): its waves
-- by ticks (Param1 0), this action (Param2), 45 ticks (Param3).
local WHIRLWIND = { 0x00, 0x3D, 0x2D, 0x00 }

-- `sub_8011C18`: 30 damage and 20 more per point of buster Attack (up to
-- 5), Wind.
function tengu_beast_charge.setup(navi: Object): number
    navi.attack_damage = weapon.charge_damage(navi, 0x1E, 0x14)
    navi.hit_param = HIT_PARAM
    navi.attack_element = WIND
    for i, p in WHIRLWIND do
        navi:set_attack_param(i, p)
    end
    return ACTION
end

-- `sub_80EE060`: the wind-up; when its animation ends, the whirlwind.
local function wind_up(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0x13)
        me:open_counter_window()
        me:set_status("using_action", true)
        me:set_status("moving", true)
        me.step_init = 4
        return
    end
    if not bit32.btest(me.sprite.frame_flags, 0x80) then
        return
    end
    me:set_animation(0x14)
    local params = { me:attack_param(1), me:attack_param(2), me:attack_param(3), me:attack_param(4) }
    local damage = int.u32(me.attack_damage + bit32.lshift(me.hit_param, 16))
    whirlwind.spawn(me, me.panel_x + me.facing, me.panel_y, me.attack_element, params, damage)
    battle.play_sound(0xB8)
    s.timer = 0x2D
    me.step = HOLD
    me.step_init = 0
end

-- `sub_80ED998`: 46 ticks, then idle.
local function hold(me: Object, s: State)
    local t = s.timer - 1
    s.timer = t
    if t < 0 then
        me:set_status("moving", false)
        me:exit_attack()
    end
end

-- `sub_80EE044`.
function tengu_beast_charge.update(me: Object, s: State)
    local step = me.step
    if step == WIND_UP then
        wind_up(me, s)
    elseif step == HOLD then
        hold(me, s)
    else
        error(string.format("TenguCross Beast charge phase %#x reads past its phase table", step))
    end
end

return tengu_beast_charge
