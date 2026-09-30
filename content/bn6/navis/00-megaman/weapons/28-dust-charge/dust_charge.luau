--!strict
-- DustCross's charged shot: weapon routine 0x28 (`sub_8011F64`) and action
-- 0x57 (`sub_80EFC1C`). The navi raises the buster, rolls a ball of junk
-- forward (objects/dust-ball) on the second tick, and idles 35 ticks later.

local buster = require("../../../../lib/buster")
local dust_ball = require("../../../../objects/dust-ball/dust_ball")

export type State = {
    -- Ticks into the shot, then ticks of recovery left (AIAttackVars+0x10).
    timer: number,
}

local dust_charge = {
    state = { timer = "u16" },
}

local ACTION = 0x57

-- The phases, on the attack's step.
local SHOOT, RECOVER = 0x0, 0x4

-- Break (secondary element 0x10), and the hit's counter byte.
local BREAK, HIT_PARAM = 0x10, 0x94

-- `sub_8011F64`: 50 damage and 10 more per point of buster Attack (up to 5)
-- (`sub_8012642`), Break.
function dust_charge.setup(navi: Object): number
    navi.attack_damage = 0x32 + 10 * math.min(navi:buster_damage(), 5)
    navi.hit_param = HIT_PARAM
    navi.attack_element = BREAK
    navi.charged = 0
    navi.extra = 0
    navi.attack_lockout = 0
    return ACTION
end

-- `sub_80EFC38`: raise the buster; the ball on the second tick, from the
-- panel in front; after 5 ticks, the recovery.
local function shoot(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0x0E)
        buster.raise_arm(me)
        me:open_counter_window()
        me:set_status("using_action", true)
        battle.play_sound(0xFF)
        me.step_init = 4
        s.timer = 0
    end
    if s.timer == 1 then
        local damage = bit32.bor(me.attack_damage, bit32.lshift(me.hit_param, 16))
        dust_ball.spawn(me, me.panel_x + me.facing, me.panel_y, me.attack_element, damage)
    end
    s.timer += 1
    if s.timer > 4 then
        me.step = RECOVER
        me.step_init = 0
    end
end

-- `sub_80EFC8E`: 31 ticks, then idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = 0x1E
        me.step_init = 4
    end
    local left = s.timer - 1
    s.timer = left
    if left < 0 then
        me.related1 = nil
        me.overlay = nil
        me:exit_attack()
    end
end

-- `sub_80EFC1C`.
function dust_charge.update(me: Object, s: State)
    local step = me.step
    if step == SHOOT then
        shoot(me, s)
    elseif step == RECOVER then
        recover(me, s)
    else
        error(string.format("DustCross's charged shot phase %#x reads past its phase table", step))
    end
end

return dust_charge
