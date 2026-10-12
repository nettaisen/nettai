--!strict
-- H-Burst, as the games share it (each game passes its `HyperBurstLook`:
-- its spreader and how the navi holds it, its sound, its shot kind and the
-- navi's animation): the navi raises a spreader (an attachment in the
-- look's slot) and on the second tick fires H-Burst's shot (./burst) along
-- its row from the panel ahead: at the first thing it hits it bursts again
-- and again. Then 21 ticks of recovery (EXE6's action 0x34, `sub_80ED810`;
-- EXE5's 0x35, the same code but for the spreader's attachment row; EXE4's
-- 0x71, 0x080EE93A, the same but for the navi's animation and its own
-- attachment in the related slot).

local burst = require("./burst")
local slot = require("../slot")

local action = {}

export type State = {
    -- Ticks into the shot, then ticks left (AIAttackVars+0x10).
    timer: number,
}

-- One state for every H-Burst action.
action.STATE = { timer = "u16" }

-- The phases, on the attack's step.
local AIM, RECOVER = 0x0, 0x4

local RECOVER_TICKS = 0x14

-- An H-Burst of a game's `look` whose shot bursts `spec.bursts` times.
function action.action(spec: HyperBurstSpec, look: HyperBurstLook): Action
    -- `sub_80ED82C`: the spreader out; on the second tick it fires.
    local function aim(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(look.anim)
            me:open_counter_window()
            look.hold(me, look.spreader, look.slot)
            me:set_status("using_action", true)
            me.step_init = 4
            s.timer = 0
        end
        local t = s.timer + 1
        s.timer = t
        if t <= 1 then
            return
        end
        -- The spreader's firing animation (with no spreader, the game's
        -- store lands in low memory and does nothing).
        local spreader = slot.held(me, look.slot)
        if spreader then
            spreader.anim += 1
        end
        battle.play_sound(look.sound)
        -- The damage word (the hit parameter in its high half), plus the
        -- Atk+ bonus.
        local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
        burst.spawn(look.burst, me, me.panel_x + me.facing, me.panel_y, me.attack_element, spec.bursts, damage)
        me.step = RECOVER
        me.step_init = 0
    end

    -- `sub_80ED896`: 21 ticks, then the spreader goes and back to idle.
    local function recover(me: Object, s: State)
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

    -- `sub_80ED810`.
    local function update(me: Object, s: State)
        local step = me.step
        if step == AIM then
            aim(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("H-Burst phase %#x reads past its phase table", step))
        end
    end

    return new.action { id = spec.id, state = action.STATE, args = spec, update = update }
end

return action
