--!strict
-- KnightSoul's charged shot (`knightsoul/charge`): the original's weapon routine 0x11 (0x0800F7A6) sets 30 damage and
-- 10 more per point of buster Attack plus one (0x0800FE5E), its own counter byte and the ball's swing (12 a tick,
-- one turn, confusing for 480 ticks, the ground ball), and starts action 0x40 (0x080EEAF6): MegaMan swings
-- (the counter window open) and on the fifth tick his wrecking ball (KnightMan's, chips/knightmn/ball: 0x080C768A,
-- not running while dimmed as KnightMan's does) circles him on the ground, a sound. 15 ticks, then 16 and 16 more
-- (his arm down).

local ball = require("../../../../chips/knightmn/ball")
local status = require("../../../../rules/status")

local charge = {}

export type State = {
    -- AIAttackVars +0x10: the step's ticks (counted up, then down).
    timer: number,
}

local HIT_PARAM = 0x9E
local SWING_ANIM, AFTER_ANIM = 16, 17
local SOUND = asset.sound("knight-swing")
local SWING_AT, SWING_TICKS, HOLD_TICKS = 4, 15, 15
-- The routine's parameters (0x0120010C): 12 a tick, one turn, status 0x20, variant 1: KnightSoul's ball, another
-- sprite, on the ground, at 35 pixels.
local SWING: KnightBallSwing = {
    speed = 12,
    turns = 1,
    status = status.confuse_480,
    look = ball.look { sprite = asset.sprite("knightsoul-ball"), z = 0, radius = 0x23 },
}

-- 0x080EEB14: the swing; the ball on the count's 4; 15 ticks.
local function swing(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(SWING_ANIM)
        me:open_counter_window()
        me:set_status("using_action", true)
        s.timer = 0
        me.step_init = 4
    end
    if s.timer == SWING_AT then
        battle.play_sound(SOUND)
        ball.spawn(me, int.u32(me.attack_damage + me.hit_param * 0x10000), SWING)
    end
    s.timer += 1
    if s.timer >= SWING_TICKS then
        me.step, me.step_init = 4, 0
    end
end

-- A wait of 16 ticks (`anim` set first if any), then step `after` (0x080EEB5C, 0x080EEB7A).
local function hold(me: Object, s: State, anim: number?, after: number?)
    if me.step_init == 0 then
        if anim then
            me:set_animation(anim)
        end
        s.timer = HOLD_TICKS
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = bit32.band(t, 0xFFFF)
    if t >= 0 then
        return
    end
    if after then
        me.step, me.step_init = after, 0
    else
        me:exit_attack()
    end
end

charge.action = new.action {
    id = "knightsoul/charge/action",
    state = { timer = "u16" },
    -- 0x080EEAF6
    update = function(me: Object, s: State)
        local step = me.step
        if step == 0 then
            swing(me, s)
        elseif step == 4 then
            hold(me, s, nil, 8)
        elseif step == 8 then
            hold(me, s, AFTER_ANIM, nil)
        else
            error(string.format("KnightSoul's charged shot's step %#x reads past its table (0x080EEB08)", step))
        end
    end,
}

-- 0x0800F7A6 (its AIAttackVars +3 1, +0x28 his second related object and +0x30 0, which the action doesn't read).
local function setup(navi: Object): Action
    navi.attack_damage = 30 + 10 * (battle.navi(navi.alliance).attack + 1)
    navi.hit_param = HIT_PARAM
    navi.marker = 0
    return charge.action
end

charge.weapon = new.weapon {
    -- The charge table's row 0x11 (0x0801CB16); Charge 5 reads on into row 0x12's first entry.
    charge_ticks = { 100, 95, 90, 85, 60, 80 },
    setup = setup,
}

return charge
