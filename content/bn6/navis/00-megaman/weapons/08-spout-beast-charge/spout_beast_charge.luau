--!strict
-- SpoutCross Beast's charged chip: weapon routine 8 (`sub_8011BE8`) and
-- action 0x3A (`sub_80EDE7E`). A charged Null-family chip becomes a surge
-- of water (objects/aqua-surge) from the panel in front, 8 ticks in; the
-- navi holds for 61 more.

local aqua_surge = require("../../../../objects/aqua-surge/aqua_surge")
local weapon = require("../../../../lib/weapon")

export type State = {
    -- Ticks to the surge, then ticks left (AIAttackVars+0x10).
    timer: number,
}

local spout_beast_charge = {
    state = { timer = "u16" },
}

local ACTION = 0x3A

-- The phases, on the attack's step.
local WIND_UP, HOLD = 0x0, 0x4

-- Aqua (the attack's element) and the hits' counter byte.
local AQUA, HIT_PARAM = 2, 0x86

-- The surge: 60 ticks (Param1), its region back every 12 (Param2).
local SURGE = { 0x3C, 0x0C, 0, 0 }

-- `sub_8011BE8`: 10 damage and 10 more per point of buster Attack (up to
-- 5), Aqua.
function spout_beast_charge.setup(navi: Object): number
    navi.attack_damage = weapon.charge_damage(navi, 0xA, 0xA)
    navi.hit_param = HIT_PARAM
    navi.attack_element = AQUA
    return ACTION
end

-- `sub_80EDE98`: 8 ticks, then the surge.
local function wind_up(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0x12)
        me:open_counter_window()
        me:set_status("using_action", true)
        me:set_status("moving", true)
        s.timer = 7
        me.step_init = 4
        return
    end
    local t = s.timer - 1
    s.timer = t
    if t >= 0 then
        return
    end
    local damage = int.u32(me.attack_damage + bit32.lshift(me.hit_param, 16))
    aqua_surge.spawn(me, me.panel_x + me.facing, me.panel_y, me.attack_element, SURGE, damage)
    battle.play_sound(0xF4)
    battle.play_sound(0xB8)
    s.timer = 0x3C
    me.step = HOLD
    me.step_init = 0
end

-- `sub_80EDEF6`: 61 ticks, then idle.
local function hold(me: Object, s: State)
    local t = s.timer - 1
    s.timer = t
    if t < 0 then
        me:set_status("moving", false)
        me:exit_attack()
    end
end

-- `sub_80EDE7E`.
function spout_beast_charge.update(me: Object, s: State)
    local step = me.step
    if step == WIND_UP then
        wind_up(me, s)
    elseif step == HOLD then
        hold(me, s)
    else
        error(string.format("SpoutCross Beast charge phase %#x reads past its phase table", step))
    end
end

return spout_beast_charge
