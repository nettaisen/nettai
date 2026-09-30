--!strict
-- AirSpin1/2/3: action 0x38 (`sub_80EDCC0`). The navi spins up a
-- whirlwind on the panel ahead: on the third tick, variant 0 (every
-- AirSpin) sets down a whirlwind (attack object #0x9B, objects/air-spin)
-- that rolls forward to the first panel it can't enter or the fourth one
-- and spins there, hitting its panel Param2 times; variant 1 sends a
-- seeking whirlwind (attack object #0xD4, objects/seeking-whirl) down onto
-- the first panel of the other side ahead. The navi is "moving" (it can't
-- be pushed) for the 16 ticks of the spin, then recovers for 6.

local air_spin = require("../../objects/air-spin/air_spin")
local seeking_whirl = require("../../objects/seeking-whirl/seeking_whirl")

export type State = {
    -- Ticks left in the current phase (AIAttackVars+0x10).
    timer: number,
}

local chip = {
    state = { timer = "u16" },
}

-- The phases, on the attack's step.
local SPIN, RECOVER = 0x0, 0x4

-- The tick (timer value) the whirlwind comes.
local SPAWN_AT = 0x0D

-- The whirlwind each variant brings (`off_80EDD54`).
type Spawner = (Object, number, number, number, { number }, number) -> Object?
local SPAWNERS: { [number]: Spawner } = { [0] = air_spin.spawn, seeking_whirl.send }

-- `sub_80EDCDC`: spin, bring the whirlwind on the timer's 0xD, then stop
-- "moving".
local function spin(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0x0C)
        me:open_counter_window()
        me:set_status("using_action", true)
        me:set_status("moving", true)
        s.timer = 0x0F
        me.step_init = 4
    end
    if s.timer == SPAWN_AT then
        local spawner = SPAWNERS[me.variant]
        if spawner == nil then
            error(string.format("AirSpin variant %d reads past off_80EDD54", me.variant))
        end
        -- The damage word (the hit parameter in its high half), plus the
        -- Atk+ bonus, and the chip's parameters as the object's.
        local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
        local params = { me:attack_param(1), me:attack_param(2), me:attack_param(3), me:attack_param(4) }
        spawner(me, me.panel_x + me.facing, me.panel_y, me.attack_element, params, damage)
    end
    local t = s.timer - 1
    s.timer = t
    if t < 0 then
        me:set_status("moving", false)
        me.step = RECOVER
        me.step_init = 0
    end
end

-- `sub_80EDD5C`: 6 ticks, then back to idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = 5
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = t
    if t < 0 then
        me:exit_attack()
    end
end

-- `sub_80EDCC0`.
function chip.update(me: Object, s: State)
    local step = me.step
    if step == SPIN then
        spin(me, s)
    elseif step == RECOVER then
        recover(me, s)
    else
        error(string.format("AirSpin phase %#x reads past its phase table", step))
    end
end

return chip
