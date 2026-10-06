--!strict
-- EXE5's launches from the raised buster arm (SideBub's action 0x1C, 0x080EC84C, and WoodNos's 0x28, 0x080ED2A0,
-- the same code): the navi raises its buster arm (animation 14) and, on the action's count 1, launches what the
-- chip sends with the attack's damage word; at 16 it recovers for 6 ticks, then lowers the arm and goes back to
-- idle.

local arm = require("./arm")

local arm_launch = {}

export type ActionState = {
    -- The action's count (AIAttackVars+0x10).
    timer: number,
}

local THROW_ANIM = 14
local LAUNCH_AT, LAUNCH_TICKS, RECOVER_TICKS = 1, 15, 5

-- The phases, on the attack's step.
local LAUNCH, RECOVER = 0x0, 0x4

-- The arm raised; on the count's 1 the launch; at 16 the recovery.
local function launching(me: Object, s: ActionState, launch: (me: Object, damage: number) -> ())
    if me.step_init == 0 then
        me:set_animation(THROW_ANIM)
        arm.raise(me)
        me:set_status("using_action", true)
        me.step_init = 4
        s.timer = 0
    end
    if s.timer == LAUNCH_AT then
        launch(me, int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra))
    end
    local t = s.timer + 1
    s.timer = t
    if t > LAUNCH_TICKS then
        me.step, me.step_init = RECOVER, 0
    end
end

-- 6 ticks, then the arm goes and back to idle.
local function recover(me: Object, s: ActionState)
    if me.step_init == 0 then
        s.timer = RECOVER_TICKS
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = t
    if t < 0 then
        me:set_status("using_action", false)
        me.related1 = nil
        me.overlay = nil
        me:exit_attack()
    end
end

-- The action launching with `launch` (the owner, the damage word); `at` names the action's phase table in errors.
function arm_launch.action(at: string, launch: (me: Object, damage: number) -> ()): Action
    return new.action {
        state = { timer = "u16" },
        update = function(me: Object, s: ActionState)
            local step = me.step
            if step == LAUNCH then
                launching(me, s, launch)
            elseif step == RECOVER then
                recover(me, s)
            else
                error(string.format("the arm launch's phase %#x reads past %s", step, at))
            end
        end,
    }
end

return arm_launch
