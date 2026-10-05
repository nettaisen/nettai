--!strict
-- KnightMan's charged attack (`knightman/charge`): the original's weapon routine 0x10 (0x0800F776) takes the damage his
-- level gives (the damage rows' row 20, lib/navi_level), his counter byte, the attack's variant 1, the ball he wears,
-- no wait and the ball's swing (12 a tick, one turn, no status, his own ball), and starts his action 9 (0x080FBFF4).
-- Two ticks in he swings (the counter window open, his swing's sound): his wrecking ball circles him once (attack
-- object #0x17, chips/knightmn/ball: KnightMan the navi chip's, here not running while dimmed), the one he wears
-- hidden meanwhile; 58 ticks, then he stands with it back.
--
-- The action is also his in an auto battle (the attack's variant 0): he glows for 20 ticks first, his guard down,
-- and waits what the attack says; its end puts his guard back up. A player's KnightMan has no guard to drop. (His
-- wait's start also calls 0x080E9FD2, the operation battle's mark on his panel.)

local ball = require("../../chips/knightmn/ball")
local chain = require("./chain")
local marker = require("../../lib/navi-chips/marker")
local navi_level = require("../../lib/navi_level")

local charge = {}

export type State = {
    -- AIAttackVars +0x10: ticks left in the current step.
    timer: number,
    -- AIAttackVars +0x30: the wait before the swing.
    wait: number,
}

local HIT_PARAM = 0x9E
-- The damage rows' row 20, by his level.
local DAMAGE: { number } = { 80, 80, 80, 100, 100, 120, 120 }
local SWING_ANIM = 18
local SOUND = asset.sound("knight-swing")
local SWING_TICKS = 58
-- The routine's ball (AIAttackVars +0x0C to +0x0F): 12 a tick, one turn, no status, his own ball.
local SWING: { speed: number, turns: number, status: Status?, variant: number } =
    { speed = 12, turns = 1, status = nil, variant = 0 }

-- The steps, on the attack's step.
local START, WAIT, SWINGING = 0x0, 0x4, 0x8

-- 0x080FC014: the action is his; the routine's variant goes straight on.
local function start(me: Object)
    me:set_status("using_action", true)
    me.step, me.step_init = WAIT, 0
end

-- 0x080FC064: the attack's wait (the routine's: none, so the same tick moves on).
local function wait(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = s.wait
        marker.spawn(me, me.panel_x, me.panel_y, s.wait + 30, 16, true)
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t < 0 then
        me.step, me.step_init = SWINGING, 0
    end
end

-- 0x080FC090: the swing: the ball round him, the one he wears hidden; 58 ticks; then he stands, it shown and let
-- go.
local function swing(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(SWING_ANIM)
        me:open_counter_window()
        s.timer = SWING_TICKS
        chain.hide(me)
        battle.play_sound(SOUND)
        ball.spawn(me, int.u32(me.attack_damage + me.hit_param * 0x10000), SWING)
        me.step_init = 4
        return
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t > 0 then
        return
    end
    me:set_animation(0)
    chain.show(me)
    chain.let_go(me)
    me:set_status("using_action", false)
    me:exit_attack()
end

charge.action = define.action {
    id = "knightman/charge/action",
    state = { timer = "u16", wait = "u16" },
    -- 0x080FBFF4
    update = function(me: Object, s: State)
        local step = me.step
        if step == START then
            start(me)
        elseif step == WAIT then
            wait(me, s)
        elseif step == SWINGING then
            swing(me, s)
        else
            error(string.format("KnightMan's charged attack's step %#x reads past its table (0x080FC008)", step))
        end
    end,
}

-- 0x0800F776
local function setup(navi: Object): Action
    navi.attack_damage = navi_level.damage(navi, DAMAGE)
    navi.hit_param = HIT_PARAM
    navi.attack_variant = 1
    local s = navi:action_state(charge.action) :: State
    s.wait = 0
    return charge.action
end

charge.weapon = define.weapon {
    id = "knightman/charge",
    -- The charge table's row 0x10 (0x0801CB0C), by Charge 0 to 5: past 4 it reads on into row 0x11's first entry.
    charge_ticks = { 80, 75, 70, 65, 60, 100 },
    setup = setup,
}

return charge
