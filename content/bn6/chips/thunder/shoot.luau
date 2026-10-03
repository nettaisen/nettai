--!strict
-- Thunder's and DarkThnd's shot (the original's action 0x1F,
-- `sub_80EC7A6`): the navi raises the buster arm and, on the second tick,
-- sets a thunder ball (ball.luau) on the panel ahead, 10 pixels up: it
-- homes in on the enemy. The navi holds the pose 5 ticks and recovers for
-- 31.

local ball = require("./ball")
local buster = require("../../lib/buster")

local shoot = {}

export type State = {
    -- Ticks of the pose so far, then the recovery's (AIAttackVars+0x10).
    timer: number,
}

-- A chip's shot: its ball; `id`, the action's key when no chip definition
-- holds it.
-- (`kind`: a game's own kind of the ball, `ball.make_kind`'s.)
export type Spec = { id: string?, ball: ThunderBall, kind: Kind? }

-- One state for both chips' shots.
shoot.STATE = { timer = "u16" }

-- The phases, on the attack's step.
local SHOOT, RECOVER = 0x0, 0x4

function shoot.action(spec: Spec): Action
    -- `sub_80EC7C0`: the arm up; the ball on the second tick; 5 ticks.
    local function shooting(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(0x0E)
            buster.raise_arm(me)
            me:open_counter_window()
            me:set_status("using_action", true)
            me.step_init = 4
            s.timer = 0
        end
        if s.timer == 1 then
            -- The damage word (the hit parameter in its high half), plus
            -- the Atk+ bonus.
            local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
            ball.spawn(me, me.panel_x + me.facing, me.panel_y, me.attack_element, damage, spec.ball, spec.kind)
        end
        local t = s.timer + 1
        s.timer = t
        if t > 4 then
            me.step = RECOVER
            me.step_init = 0
        end
    end

    -- `sub_80EC81E`: 31 ticks, then the arm goes and back to idle.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            s.timer = 0x1E
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

    -- `sub_80EC7A6`.
    local function update(me: Object, s: State)
        local step = me.step
        if step == SHOOT then
            shooting(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("Thunder phase %#x reads past its phase table", step))
        end
    end

    return define.action { id = spec.id, state = shoot.STATE, args = spec, update = update }
end

return shoot
