--!strict
-- CopyDmg: action 0x23 (`sub_80ECA34`). The navi raises its buster and, on
-- the next tick, fires CopyDmg's mark (attack object #0x28,
-- objects/copy-mark) along its row from the panel ahead: the first navi it
-- hits takes, for 180 ticks, the damage its side took the tick before on
-- top of its own. 5 ticks, then 11 of recovery.

local buster = require("../../lib/buster")
local copy_mark = require("../../objects/copy-mark/copy_mark")

export type State = {
    -- Ticks into the shot, then ticks left (AIAttackVars+0x10).
    timer: number,
}

local copy_dmg = {
    state = { timer = "u16" },
}

-- The phases, on the attack's step.
local SHOOT, RECOVER = 0x0, 0x4

-- `sub_80ECA50`: the buster up; the mark flies on the second tick.
local function shoot(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0x0E)
        buster.raise_arm(me)
        me:open_counter_window()
        me.step_init = 4
        s.timer = 0
    end
    if s.timer == 1 then
        battle.play_sound(0x6A)
        -- The damage word (the hit parameter in its high half), plus the
        -- Atk+ bonus.
        local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
        local params = { me:attack_param(1), me:attack_param(2), me:attack_param(3), me:attack_param(4) }
        copy_mark.spawn(me, me.panel_x + me.facing, me.panel_y, me.attack_element, params, damage)
    end
    local t = s.timer + 1
    s.timer = t
    if t > 4 then
        me.step = RECOVER
        me.step_init = 0
    end
end

-- `sub_80ECAA4`: 11 ticks, then the arm goes and back to idle.
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

-- `sub_80ECA34`.
function copy_dmg.update(me: Object, s: State)
    local step = me.step
    if step == SHOOT then
        shoot(me, s)
    elseif step == RECOVER then
        recover(me, s)
    else
        error(string.format("CopyDmg phase %#x reads past its phase table", step))
    end
end

return copy_dmg
