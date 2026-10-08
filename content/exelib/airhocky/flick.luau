--!strict
-- The hockey chips' flick (EXE6's action 0x26, `sub_80ECCB0`: AirHocky's and PitHocky's; EXE5's action 0x21,
-- 0x080ECCB4: AirHoc's): the navi flicks a puck (@exelib/airhocky/puck) from the panel ahead on the third tick,
-- slanting down the field or up; it bounces around the field, a panel per 6 ticks. 31 ticks in all.
--
-- EXE5's recovery sets its 16 ticks when it starts (0x080ECD28) where EXE6's flick sets them as it ends: the same
-- ticks. EXE4's (action 0x27, 0x080ECEAE) is EXE5's but for the navi's animation (its 0x10: `anim`).

local puck = require("./puck")

local flick = {}

export type State = {
    -- Ticks left in the current phase (AIAttackVars+0x10).
    timer: number,
}

-- One state for every flick.
flick.STATE = { timer = "u16" }

-- The phases, on the attack's step.
local FLICK, RECOVER = 0x0, 0x4
-- The navi's animation as it flicks (EXE6's and EXE5's).
local ANIM = 0x0C
-- The tick (timer value) the puck goes.
local FLICK_AT = 0x0D
-- The puck's height at the spawn (the registers; its init lands it).
local HEIGHT = 0x0A_0000

-- A flick: the action's key `id`, its puck's kind and variant, and whether it goes down the field.
export type Spec = FlickSpec

function flick.action(spec: Spec): Action
    -- `sub_80ECCCC`: flick the puck on the timer's 0xD, then recover.
    local function flicking(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(spec.anim or ANIM)
            me:open_counter_window()
            me:set_status("using_action", true)
            me.step_init = 4
            s.timer = 0x0F
        end
        if s.timer == FLICK_AT then
            -- The damage word (the hit parameter in its high half), plus the Atk+ bonus.
            local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
            puck.spawn(spec.kind, me, me.panel_x + me.facing, me.panel_y, me.attack_element, HEIGHT, damage, spec.puck, spec.down)
        end
        local t = s.timer - 1
        s.timer = t
        if t <= 0 then
            s.timer = 0x0F
            me.step = RECOVER
            me.step_init = 0
        end
    end

    -- `sub_80ECA0C`: 16 ticks, then back to idle.
    local function recover(me: Object, s: State)
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            me:exit_attack()
        end
    end

    -- `sub_80ECCB0`.
    local function update(me: Object, s: State)
        local step = me.step
        if step == FLICK then
            flicking(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("the flick's phase %#x reads past its phase table", step))
        end
    end

    return new.action { id = spec.id, state = flick.STATE, args = spec, update = update }
end

return flick
