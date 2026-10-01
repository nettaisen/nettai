--!strict
-- DustCross's charged shot (`dustcross/charge`): the original's weapon
-- routine 0x28 (`sub_8011F64`) and action 0x57 (`sub_80EFC1C`). The navi
-- raises the buster, rolls a ball of junk forward (./junk_ball) on the
-- second tick, and idles 35 ticks later.

local buster = require("../../../../lib/buster")
local junk_ball = require("./junk_ball")

export type State = {
    -- Ticks into the shot, then ticks of recovery left (AIAttackVars+0x10).
    timer: number,
}

local charge = {}

-- The phases, on the attack's step.
local SHOOT, RECOVER = 0x0, 0x4

-- Break (secondary element 0x10), and the hit's counter byte.
local BREAK, HIT_PARAM = 0x10, 0x94

local SOUND = asset.sound("junk-shot")

-- `sub_8011F64`: 50 damage and 10 more per point of buster Attack (up to 5)
-- (`sub_8012642`), Break.
local function setup(navi: Object): Action
    navi.attack_damage = 0x32 + 10 * math.min(navi:buster_damage(), 5)
    navi.hit_param = HIT_PARAM
    navi.attack_element = BREAK
    navi.charged = 0
    navi.extra = 0
    navi.attack_lockout = 0
    return charge.action
end

-- `sub_80EFC38`: raise the buster; the ball on the second tick, from the
-- panel in front; after 5 ticks, the recovery.
local function shoot(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0x0E)
        buster.raise_arm(me)
        me:open_counter_window()
        me:set_status("using_action", true)
        battle.play_sound(SOUND)
        me.step_init = 4
        s.timer = 0
    end
    if s.timer == 1 then
        local damage = bit32.bor(me.attack_damage, bit32.lshift(me.hit_param, 16))
        junk_ball.spawn(me, me.panel_x + me.facing, me.panel_y, me.attack_element, damage)
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

charge.action = define.action {
    id = "dustcross/charge/action",
    state = { timer = "u16" },
    -- `sub_80EFC1C`.
    update = function(me: Object, s: State)
        local step = me.step
        if step == SHOOT then
            shoot(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("DustCross's charged shot phase %#x reads past its phase table", step))
        end
    end,
}

charge.weapon = define.weapon {
    id = "dustcross/charge",
    name = "Dust charge",
    -- The charge table's row 0x28, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 80, 70, 60, 55, 50, 17, 17, 17 },
    setup = setup,
    -- (Transitional: the routine number the pack's forms name it by.)
    legacy = { routines = { 0x28 } },
}

return charge
