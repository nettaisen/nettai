--!strict
-- AirSpin's action (BN6's 0x38, `sub_80EDCC0`; BN5's 0x47, 0x080EF030, the
-- same code): the navi spins up a whirlwind on the panel ahead. On the
-- third tick it sends what its chip composes (`action { whirlwind = ... }`,
-- the original's variant and parameters): every AirSpin a whirlwind top
-- (./top) that rolls forward to the first panel it can't enter or the
-- fourth one and spins there; BN6's variant no chip has, a seeking
-- whirlwind (bn6 chips/airspin/whirl); BN5's AqWhirl a whirlpool. The navi
-- is "moving" (it can't be pushed) for the 16 ticks of the spin, then
-- recovers for 6.

local action = {}

export type State = {
    -- Ticks left in the current phase (AIAttackVars+0x10).
    timer: number,
}

-- One state for every AirSpin: a use continues the last one's attack
-- variables, as the original's one action does.
action.STATE = { timer = "u16" }

-- The phases, on the attack's step.
local SPIN, RECOVER = 0x0, 0x4

local ANIM_SPIN = 0x0C
-- The phase's ticks, and the tick (timer value) the whirlwind comes.
local SPIN_TICKS, SPAWN_AT = 0x0F, 0x0D
local RECOVER_TICKS = 5

-- A chip's AirSpin: what it sends.
function action.action(spec: AirSpinSpec): Action
    -- `sub_80EDCDC`: spin, bring the whirlwind on the timer's 0xD, then
    -- stop "moving".
    local function spin(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(ANIM_SPIN)
            me:open_counter_window()
            me:set_status("using_action", true)
            me:set_status("moving", true)
            s.timer = SPIN_TICKS
            me.step_init = 4
        end
        if s.timer == SPAWN_AT then
            -- The damage word (the hit parameter in its high half), plus
            -- the Atk+ bonus.
            local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
            spec.whirlwind.send(me, me.panel_x + me.facing, me.panel_y, me.attack_element, damage)
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
            s.timer = RECOVER_TICKS
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            me:exit_attack()
        end
    end

    -- `sub_80EDCC0`.
    local function update(me: Object, s: State)
        local step = me.step
        if step == SPIN then
            spin(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("AirSpin phase %#x reads past its phase table", step))
        end
    end

    return define.action { id = spec.id, state = action.STATE, args = spec, update = update }
end

return action
