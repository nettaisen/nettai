--!strict
-- EXE5's arm shots (actions 0x51 and 0x52, 0x080EF64A and 0x080EF6E2: SpShake's and Pulsar's, EXE5's own, the same
-- code but for what they fire): the navi raises its buster arm (lib/arm) and on the second tick fires from the
-- panel ahead; 6 ticks in all, then 5 to stand down.

local arm = require("./arm")

local armshot = {}

export type State = {
    -- Ticks into the shot, then of the recovery (AIAttackVars+0x10).
    timer: number,
}

armshot.STATE = { timer = "u16" }

local FIRE, RECOVER = 0x0, 0x4
local ANIM = 0x0E
local FIRE_AT, FIRE_TICKS, RECOVER_TICKS = 1, 4, 5

-- A chip's shot: the action's key, and what it fires from (x, y) with the damage word.
export type Spec = { id: string?, fire: (me: Object, x: number, y: number, damage: number) -> () }

function armshot.action(spec: Spec): Action
    local function fire(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(ANIM)
            arm.raise(me)
            me:set_status("using_action", true)
            me.step_init = 4
            s.timer = 0
        end
        if s.timer == FIRE_AT then
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
            spec.fire(me, me.panel_x + me.facing, me.panel_y, damage)
        end
        local t = s.timer + 1
        s.timer = t
        if t > FIRE_TICKS then
            me.step, me.step_init = RECOVER, 0
        end
    end

    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            s.timer = RECOVER_TICKS
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = t
        if t >= 0 then
            return
        end
        me:set_status("using_action", false)
        me.related1 = nil
        me.overlay = nil
        me:exit_attack()
    end

    return new.action {
        id = spec.id,
        state = armshot.STATE,
        args = spec,
        update = function(me: Object, s: State)
            local step = me.step
            if step == FIRE then
                fire(me, s)
            elseif step == RECOVER then
                recover(me, s)
            else
                error(string.format("arm shot phase %#x reads past its phase table", step))
            end
        end,
    }
end

return armshot
