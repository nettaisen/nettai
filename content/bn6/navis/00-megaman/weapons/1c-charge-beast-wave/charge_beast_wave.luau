--!strict
-- ChargeCross Beast's charged Null chip: weapon routine 0x1C (`sub_8011DDC`)
-- and action 0x4F (`sub_80EF282`). The navi winds up for 31 ticks, twice
-- striking the whole field with a harmless hit carrying status 0x46; then
-- a wave rolls out from its panel (objects/charge-wave), and 51 ticks
-- later it idles.

local weapon = require("../../../../lib/weapon")
local charge_wave = require("../../../../objects/charge-wave/charge_wave")

export type State = {
    -- AIAttackVars+0x10: ticks of the phase.
    timer: number,
}

local charge_beast_wave = {
    state = { timer = "u16" },
}

local ACTION = 0x4F

-- The phases, on the attack's step.
local WIND_UP, RECOVER = 0x0, 0x4

-- Fire, the hit's counter byte.
local FIRE, HIT_PARAM = 1, 0x9E

-- How high the wave rolls, 16.16.
local WAVE_Z = 0x18_0000

-- `sub_8011DDC`: 70 damage and 30 more per point of buster Attack (up to
-- 5), Fire.
function charge_beast_wave.setup(navi: Object): number
    navi.charged = 0
    navi.attack_lockout = 0
    navi.extra = 0
    navi.hit_param = HIT_PARAM
    navi.attack_damage = weapon.charge_damage(navi, 0x46, 0x1E)
    navi.attack_element = FIRE
    return ACTION
end

-- `sub_80EF30A`: a harmless hit on every panel (region 0x83) with
-- status 0x46, element 5.
local function field_hit(me: Object)
    battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = 5,
        z = 0,
        region = 0x83,
        hit_effect = 0xFF,
        target = 5,
        self_type = 0x32,
        damage = 0,
        stamina = 0,
        hit_mod = 0,
        status = 0x46,
    })
end

-- `sub_80EF29C`: 31 ticks of wind-up, with the field hit at its start and
-- halfway; then the wave.
local function wind_up(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0x12)
        me:open_counter_window()
        me:set_status("using_action", true)
        me:set_status("moving", true)
        field_hit(me)
        s.timer = 0x1E
        me.step_init = 4
    end
    if s.timer == 0xF then
        field_hit(me)
    end
    local left = s.timer - 1
    s.timer = left
    if left >= 0 then
        return
    end
    me:set_animation(0x13)
    local damage = bit32.bor(me.attack_damage, bit32.lshift(me.hit_param, 16))
    charge_wave.spawn(me, me.panel_x + me.facing, me.panel_y, me.attack_element, damage, WAVE_Z)
    battle.play_sound(0x1C7)
    s.timer = 0x32
    me.step = RECOVER
    me.step_init = 0
end

-- `sub_80ED998`: 51 ticks, then idle.
local function recover(me: Object, s: State)
    local left = s.timer - 1
    s.timer = left
    if left >= 0 then
        return
    end
    me:set_status("moving", false)
    me:exit_attack()
end

-- `sub_80EF282`.
function charge_beast_wave.update(me: Object, s: State)
    local step = me.step
    if step == WIND_UP then
        wind_up(me, s)
    elseif step == RECOVER then
        recover(me, s)
    else
        error(string.format("ChargeCross Beast's wave phase %#x reads past its phase table (sub_80EF282)", step))
    end
end

return charge_beast_wave
