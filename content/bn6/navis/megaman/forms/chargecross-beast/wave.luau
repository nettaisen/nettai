--!strict
-- ChargeCross Beast's charged Null chip (`chargecross-beast/wave`): the
-- original's weapon routine 0x1C (`sub_8011DDC`) and action 0x4F
-- (`sub_80EF282`). The navi winds up for 31 ticks, twice striking every
-- solid panel with a hit of no damage and status 0x46; then a wave rolls
-- out from its panel (./charge_wave), and 51 ticks later it idles.

local charge_wave = require("./charge_wave")
local collision = require("../../../../rules/collision")
local regions = require("@exelib/regions")
local statuses = require("../../../../rules/status")
local weapon = require("../../../../lib/weapon")

export type State = {
    -- AIAttackVars+0x10: ticks of the phase.
    timer: number,
}

local wave = {}

-- The phases, on the attack's step.
local WIND_UP, RECOVER = 0x0, 0x4

-- Fire, the hit's counter byte.
local FIRE, HIT_PARAM = 1, 0x9E

local SOUND = asset.sound("ground-beast-dash-2")

-- How high the wave rolls, 16.16.
local WAVE_Z = 0x18_0000

-- The wind-up's hit over the field: no damage, element 5, a guard-breaking
-- attack at navis with status 0x46, no spark.
local FIELD_ELEMENT, FIELD_STATUS = 5, statuses.immobilize_30

-- `sub_8011DDC`: 70 damage and 30 more per point of buster Attack (up to
-- 5), Fire.
local function setup(navi: Object): Action
    navi.charged = 0
    navi.attack_lockout = 0
    navi.extra = 0
    navi.hit_param = HIT_PARAM
    navi.attack_damage = weapon.charge_damage(navi, 0x46, 0x1E)
    navi.attack_element = FIRE
    return wave.action
end

-- `sub_80EF30A`: a hit of no damage on every solid panel, with its status
-- and element.
local function field_hit(me: Object)
    battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = FIELD_ELEMENT,
        z = 0,
        region = regions.solid,
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.guard_breaking,
        damage = 0,
        stamina = 0,
        hit_mod = 0,
        status = FIELD_STATUS,
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
    battle.play_sound(SOUND)
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

wave.action = define.action {
    id = "chargecross-beast/wave/action",
    state = { timer = "u16" },
    -- `sub_80EF282`.
    update = function(me: Object, s: State)
        local step = me.step
        if step == WIND_UP then
            wind_up(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("ChargeCross Beast's wave phase %#x reads past its phase table (sub_80EF282)", step))
        end
    end,
}

wave.weapon = define.weapon {
    id = "chargecross-beast/wave",
    -- The charge table's row 0x1C, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 20, 20, 20, 20, 20, 40, 40, 40 },
    setup = setup,
}

return wave
