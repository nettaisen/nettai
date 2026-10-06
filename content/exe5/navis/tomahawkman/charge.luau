--!strict
-- TomahawkMan's charged attack (`tomahawkman/charge`): the original's weapon routine 0x3C (0x0800FACA) sets the damage
-- his level gives (the damage rows' row 18, lib/navi_level), his own counter byte, no element and a 30-tick wait, and
-- starts his action 7 (0x080F1760): he throws his tomahawk (./tomahawk) and holds the throw 10 ticks, then stands
-- out the wait. No counter window opens, and he can be moved off it: the action sets no status.

local navi_level = require("../../lib/navi_level")
local tomahawk = require("./tomahawk")

local charge = {}

export type State = {
    -- AIAttackVars +0x10: ticks left in the current step.
    timer: number,
    -- AIAttackVars +0x18: the ticks he stands after the throw (the routine's).
    wait: number,
}

local HIT_PARAM = 0x94
-- The damage rows' row 18, by his level.
local DAMAGE: { number } = { 60, 60, 60, 60, 60, 80, 80 }
local THROW_ANIM = 12
local THROW_TICKS, WAIT = 10, 30

-- 0x080F177C: the throw; the tomahawk leaves on its first tick; 10 ticks.
local function throw(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = THROW_ANIM
        tomahawk.throw(me, me.attack_element, int.u32(me.attack_damage + me.hit_param * 0x10000))
        s.timer = THROW_TICKS
        return
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t <= 0 then
        me.step, me.step_init = 4, 0
    end
end

-- 0x080F17AC: standing, the routine's wait; then back to idle.
local function stand(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = 0
        s.timer = s.wait
        return
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t <= 0 then
        me:exit_attack()
    end
end

charge.action = new.action {
    id = "tomahawkman/charge/action",
    state = { timer = "u16", wait = "u16" },
    -- 0x080F1760
    update = function(me: Object, s: State)
        local step = me.step
        if step == 0 then
            throw(me, s)
        elseif step == 4 then
            stand(me, s)
        else
            error(string.format("TomahawkMan's charged attack's step %#x reads past its table (0x080F1774)", step))
        end
    end,
}

-- 0x0800FACA
local function setup(navi: Object): Action
    navi.attack_damage = navi_level.damage(navi, DAMAGE)
    navi.hit_param = HIT_PARAM
    navi.attack_element = 0
    local s = navi:action_state(charge.action) :: State
    s.wait = WAIT
    return charge.action
end

charge.weapon = new.weapon {
    id = "tomahawkman/charge",
    -- The charge table's row 0x3C (0x0801CCC4), by Charge 0 to 5: past 4 it reads on into row 0x3D's first entry.
    charge_ticks = { 80, 75, 70, 65, 60, 130 },
    setup = setup,
}

return charge
