--!strict
-- WindRack: action 0x3F (`sub_80EE192`). The navi swings a rack (an
-- attachment, kind 0x2A, or 0x33 for a variant other than 0) with a gust
-- of wind around it (effect 0x44). On the ninth tick it hits the column
-- ahead (region 4, 16 pixels up) for 10 ticks, and blows an invisible gust
-- (attack object #0x49, objects/gust) down each row from that column,
-- pushing what it meets. 22 ticks, then 6 of recovery.

local attachment = require("../../objects/attachment/attachment")
local arm = require("../../lib/arm")
local gust = require("../../objects/gust/gust")

export type State = {
    -- Ticks left in the current phase (AIAttackVars+0x10).
    timer: number,
}

local wind_rack = {
    state = { timer = "u16" },
}

-- The phases, on the attack's step.
local SWING, RECOVER = 0x0, 0x4

-- The rack (attachment kinds) for variant 0 and the others.
local RACK, OTHER_RACK = 0x2A, 0x33
-- The wind swirl effect, 16 pixels up on the navi's panel.
local SWIRL = 0x44
local HEIGHT = 0x10_0000
-- The tick (timer value) the swing lands.
local HIT_AT = 0x0C
-- The hit: region 4 (the column), no spark, collision types 5 and 0x12,
-- hit modifier 0x49, for 10 ticks.
local HIT = { region = 4, hit_effect = 0xFF, target = 5, self_type = 0x12, hit_mod = 0x49 }
local HIT_TICKS = 0x0A
-- The gusts' Param1: invisible, blowing anywhere.
local GUST_PARAMS = { 0x80 }

-- `sub_80EE1AC`: the swing; on the timer's 0xC the hit and the gusts.
local function swing(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(5)
        me:open_counter_window()
        battle.play_sound(0x11F)
        local kind = if me.variant == 0 then RACK else OTHER_RACK
        attachment.spawn(me, { kind = kind, anim = arm.anim(me) }, "related")
        me:set_status("using_action", true)
        local flip = bit32.bxor(me.alliance, me.flip)
        local cx, cy = field.center(me.panel_x, me.panel_y)
        battle.effect(Vec3.new(cx, cy, HEIGHT), SWIRL, flip)
        s.timer = 0x15
        me.step_init = 4
    end
    if s.timer == HIT_AT then
        local x = me.panel_x + me.facing
        -- The damage word (the hit parameter in its high half), plus the
        -- Atk+ bonus.
        local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
        local hit = battle.hitbox(me, {
            panel_x = x,
            panel_y = me.panel_y,
            element = me.attack_element,
            z = HEIGHT,
            region = HIT.region,
            hit_effect = HIT.hit_effect,
            target = HIT.target,
            self_type = HIT.self_type,
            damage = damage,
            stamina = bit32.rshift(damage, 16),
            hit_mod = HIT.hit_mod,
        })
        if hit then
            hit.timer = HIT_TICKS
        end
        for y = 1, 3 do
            gust.spawn(x, y, me.alliance, me.flip, GUST_PARAMS, 0, nil, 0)
        end
    end
    local t = s.timer - 1
    s.timer = t
    if t < 0 then
        me.step = RECOVER
        me.step_init = 0
    end
end

-- `sub_80EE274`: 6 ticks, then the rack goes and back to idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = 5
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = t
    if t < 0 then
        me.related1 = nil
        me.overlay = nil
        me:exit_attack()
    end
end

-- `sub_80EE192`.
function wind_rack.update(me: Object, s: State)
    local step = me.step
    if step == SWING then
        swing(me, s)
    elseif step == RECOVER then
        recover(me, s)
    else
        error(string.format("WindRack phase %#x reads past its phase table", step))
    end
end

return wind_rack
