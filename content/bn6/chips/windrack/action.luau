--!strict
-- WindRack's action (the original's 0x3F, `sub_80EE192`): the navi swings
-- a rack with a swirl of wind around it. On the ninth tick it hits the
-- column ahead, 16 pixels up, for 10 ticks, and blows an unseen gust
-- (attack object #0x49, objects/gust) down each row from that column,
-- pushing what it meets. 22 ticks, then 6 of recovery. What it swings is
-- its user's (`action { rack = ... }`, the original's variant): WindRack's
-- rack, or TenguCross's fan for its charged shot.

local attachment = require("../../objects/attachment/attachment")
local arm = require("../../lib/arm")
local collision = require("../../rules/collision")
local gust = require("../../objects/gust/gust")
local regions = require("../../lib/regions")

local action = {}

export type State = {
    -- Ticks left in the current phase (AIAttackVars+0x10).
    timer: number,
}

-- One state for every WindRack: a use continues the last one's attack
-- variables, as the original's one action does.
action.STATE = { timer = "u16" }

-- What the navi swings: WindRack's rack (the original's attachment row
-- 0x2A, variant 0) and TenguCross's fan (row 0x33, the other variants).
action.racks = {
    wind_rack = attachment.look { sprite = asset.sprite("wind-rack"), palette = 0 },
    tengu_fan = attachment.look { sprite = asset.sprite("tengu-fan"), palette = 0 },
}

-- The phases, on the attack's step.
local SWING, RECOVER = 0x0, 0x4

local ANIM_SWING = 5
local SOUND = asset.sound("windrack")
-- The swirl around the navi (effect 0x44), 16 pixels up on its panel.
local SWIRL = define.effect { sprite = asset.sprite("swirl"), anim = 0, palette = 0 }
local HEIGHT = 0x10_0000
-- The phase's ticks, and the tick (timer value) the swing lands.
local SWING_TICKS, HIT_AT = 0x15, 0x0C
local RECOVER_TICKS = 5
-- The hit: the column, no spark, a pushing hit with hit modifier 0x49,
-- for 10 ticks.
local HIT_MOD, HIT_TICKS = 0x49, 0x0A
-- The gusts: unseen, blowing anywhere (their first parameter, 0x80).
local UNSEEN: GustHow = { unseen = true }

-- A user's WindRack: what it swings.
function action.action(spec: WindRackSpec): Action
    -- `sub_80EE1AC`: the swing; on the timer's 0xC the hit and the gusts.
    local function swing(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(ANIM_SWING)
            me:open_counter_window()
            battle.play_sound(SOUND)
            attachment.attach(me, spec.rack, "related", { anim = arm.anim(me) })
            me:set_status("using_action", true)
            local flip = bit32.bxor(me.alliance, me.flip)
            local cx, cy = field.center(me.panel_x, me.panel_y)
            battle.effect(Vec3.new(cx, cy, HEIGHT), SWIRL, flip)
            s.timer = SWING_TICKS
            me.step_init = 4
        end
        if s.timer == HIT_AT then
            local x = me.panel_x + me.facing
            -- The damage word (the hit parameter in its high half), plus
            -- the Atk+ bonus.
            local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
            local hit = battle.hitbox(me, {
                panel_x = x,
                panel_y = me.panel_y,
                element = me.attack_element,
                z = HEIGHT,
                region = regions.wide,
                -- None.
                hit_effect = nil,
                target = collision.hits_navis,
                self_type = collision.pushing,
                damage = damage,
                stamina = bit32.rshift(damage, 16),
                hit_mod = HIT_MOD,
            })
            if hit then
                hit.timer = HIT_TICKS
            end
            for y = 1, 3 do
                gust.spawn(x, y, me.alliance, me.flip, UNSEEN, 0, nil, 0)
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
            s.timer = RECOVER_TICKS
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
    local function update(me: Object, s: State)
        local step = me.step
        if step == SWING then
            swing(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("WindRack phase %#x reads past its phase table", step))
        end
    end

    return define.action { id = spec.id, state = action.STATE, args = spec, update = update }
end

return action
