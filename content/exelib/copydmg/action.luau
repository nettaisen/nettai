--!strict
-- CopyDmg's action (EXE6's action 0x23, `sub_80ECA34`; EXE5's 0x24,
-- 0x080ECEFE), made for a game (`action.make`): the navi raises its buster
-- and, on the next tick, fires what its chip sends (EXE6's mark, EXE5's by
-- the chip's subtype from 0x080ECF78: 0 the mark, 1 a sword wave, which no
-- chip sends) along its row from the panel ahead. 5 ticks, then 11 of
-- recovery.

local action = {}

export type State = {
    -- Ticks into the shot, then ticks left (AIAttackVars+0x10).
    timer: number,
}

-- The phases, on the attack's step.
local SHOOT, RECOVER = 0x0, 0x4

-- A game's CopyDmg action.
function action.make(spec: CopyDamageSpec): Action
    -- `sub_80ECA50` (EXE5's 0x080ECF18): the buster up; the shot flies on
    -- the second tick.
    local function shoot(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(0x0E)
            spec.raise_arm(me)
            me:open_counter_window()
            me.step_init = 4
            s.timer = 0
        end
        if s.timer == 1 then
            battle.play_sound(spec.sound)
            -- The damage word (the hit parameter in its high half), plus
            -- the Atk+ bonus.
            local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
            spec.shot(me, me.panel_x + me.facing, me.panel_y, me.attack_element, damage)
        end
        local t = s.timer + 1
        s.timer = t
        if t > 4 then
            me.step = RECOVER
            me.step_init = 0
        end
    end

    -- `sub_80ECAA4` (EXE5's 0x080ECF84): 11 ticks, then the arm goes and
    -- back to idle.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            s.timer = 0x0A
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

    -- `sub_80ECA34` (EXE5's 0x080ECEFE).
    local function update(me: Object, s: State)
        local step = me.step
        if step == SHOOT then
            shoot(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("CopyDmg phase %#x reads past its phase table", step))
        end
    end

    return define.action { id = spec.id, state = { timer = "u16" }, update = update }
end

return action
