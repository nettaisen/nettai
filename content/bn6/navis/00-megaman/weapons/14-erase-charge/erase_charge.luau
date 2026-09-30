--!strict
-- EraseCross's charged shot: weapon routine 0x14 (`sub_8011CF8`) and action
-- 0x45 (`sub_80EE7D0`). Once the navi is still, it fires its beam
-- (objects/erase-ray) from the panel in front and holds the pose for 71
-- ticks; the beam lasts as long as the navi stays in this action. In the
-- Beast form the beam is of its second kind, 4 pixels lower.

local erase_ray = require("../../../../objects/erase-ray/erase_ray")
local weapon = require("../../../../lib/weapon")

export type State = {
    -- AIAttackVars+0x10: ticks of the pose left.
    timer: number,
}

local erase_charge = {
    state = { timer = "u16" },
}

local ACTION = 0x45

-- The phases, on the attack's step.
local FIRE, HOLD = 0x0, 0x4

-- EraseCross's form; its hit parameter.
local ERASE_CROSS, HIT_PARAM = 4, 0x94

-- `sub_8011CF8`: 40 damage and 20 more per point of buster Attack (up to
-- 5), Null; the beam's kind: 0 in EraseCross, 1 in any other form (its
-- Beast form).
function erase_charge.setup(navi: Object): number
    navi.attack_damage = weapon.charge_damage(navi, 0x28, 0x14)
    navi.hit_param = HIT_PARAM
    navi.variant = if battle.navi(navi.alliance).form == ERASE_CROSS then 0 else 1
    navi.attack_element = 0
    return ACTION
end

-- `sub_80EE7EC`: once neither moving nor sliding, the beam.
local function fire(me: Object, s: State)
    if me:status("moving") or me:status("sliding") then
        return
    end
    me:set_animation(0xF)
    me:open_counter_window()
    me:set_status("using_action", true)
    me:set_status("moving", true)
    local kind = me.variant
    local z = int.shl(-4 * kind, 16)
    local damage = bit32.bor(me.attack_damage, bit32.lshift(me.hit_param, 16))
    erase_ray.spawn(me, me.panel_x + me.facing, me.panel_y, me.attack_element, z, { kind, 0, 0, 0 }, damage)
    battle.play_sound(0x111)
    s.timer = 0x46
    me.step = HOLD
    me.step_init = 0
end

-- `sub_80ED998`: hold, then idle.
local function hold(me: Object, s: State)
    local t = s.timer - 1
    s.timer = t
    if t >= 0 then
        return
    end
    me:set_status("moving", false)
    me:exit_attack()
end

-- `sub_80EE7D0`.
function erase_charge.update(me: Object, s: State)
    local step = me.step
    if step == FIRE then
        fire(me, s)
    elseif step == HOLD then
        hold(me, s)
    else
        error(string.format("EraseCross's charged shot phase %#x reads past its phase table (sub_80EE7D0)", step))
    end
end

return erase_charge
