--!strict
-- SpoutCross Beast's charged chip (`spoutcross-beast/charge`): the
-- original's weapon routine 8 (`sub_8011BE8`) and action 0x3A
-- (`sub_80EDE7E`). A charged Null-family chip becomes a surge of water
-- (./surge) from the panel in front, 8 ticks in; the navi holds for 61
-- more.

local surge = require("./surge")
local weapon = require("../../../../lib/weapon")

export type State = {
    -- Ticks to the surge, then ticks left (AIAttackVars+0x10).
    timer: number,
}

local charge = {}

-- The phases, on the attack's step.
local WIND_UP, HOLD = 0x0, 0x4

-- Aqua (the attack's element) and the hits' counter byte.
local AQUA, HIT_PARAM = 2, 0x86

local SOUND_SURGE, SOUND_WATER = asset.sound("bn6:spout-beast-charge"), asset.sound("bn6:aqua-surge")

-- The surge: 60 ticks, its region back every 12.
local SURGE: AquaSurge = { ticks = 0x3C, beat = 0x0C }

-- `sub_8011BE8`: 10 damage and 10 more per point of buster Attack (up to
-- 5), Aqua.
local function setup(navi: Object): Action
    navi.attack_damage = weapon.charge_damage(navi, 0xA, 0xA)
    navi.hit_param = HIT_PARAM
    navi.attack_element = AQUA
    return charge.action
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
    surge.spawn(me, me.panel_x + me.facing, me.panel_y, me.attack_element, SURGE, damage)
    battle.play_sound(SOUND_SURGE)
    battle.play_sound(SOUND_WATER)
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

charge.action = define.action {
    id = "bn6:spoutcross-beast/charge/action",
    state = { timer = "u16" },
    -- `sub_80EDE7E`.
    update = function(me: Object, s: State)
        local step = me.step
        if step == WIND_UP then
            wind_up(me, s)
        elseif step == HOLD then
            hold(me, s)
        else
            error(string.format("SpoutCross Beast charge phase %#x reads past its phase table", step))
        end
    end,
}

charge.weapon = define.weapon {
    id = "bn6:spoutcross-beast/charge",
    -- The charge table's row 8, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 20, 20, 20, 20, 20, 110, 110, 110 },
    setup = setup,
}

return charge
