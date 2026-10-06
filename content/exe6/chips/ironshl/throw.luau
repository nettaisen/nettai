--!strict
-- IronShl's and ParaShl's throw (the original's action 0x2C,
-- `sub_80ED25C`): the navi throws an iron shell (shell.luau) onto the panel
-- ahead on the third tick; it slides to the far column, bumps there and
-- flies off. ParaShl throws three instead, down its own back column,
-- which slide across and back. 32 ticks in all.

local shell = require("./shell")

local throw = {}

export type State = {
    -- Ticks left in the current phase (AIAttackVars+0x10).
    timer: number,
}

-- One state for every throw.
throw.STATE = { timer = "u16" }

-- The phases, on the attack's step.
local THROW, RECOVER = 0x0, 0x4
-- The tick (timer value) the shells are thrown.
local THROW_AT = 0x0D

-- A chip's throw: its shell; `id`, the action's key (its compat key: the
-- chips are records).
function throw.action(spec: { id: string, shell: IronShell }): Action
    -- `sub_80ED278`: throw on the timer's 0xD, then recover.
    local function throwing(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(0x0C)
            me:open_counter_window()
            me:set_status("using_action", true)
            s.timer = 0x0F
            me.step_init = 4
        end
        if s.timer == THROW_AT then
            -- The damage word (the hit parameter in its high half), plus
            -- the Atk+ bonus.
            local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
            local element = me.attack_element
            if spec.shell.para then
                -- Down the navi's own back column.
                local x = me.alliance * 5 + 1
                for y = 1, 3 do
                    shell.spawn(me, x, y, element, damage, spec.shell)
                end
            else
                shell.spawn(me, me.panel_x + me.facing, me.panel_y, element, damage, spec.shell)
            end
        end
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
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

    -- `sub_80ED25C`.
    local function update(me: Object, s: State)
        local step = me.step
        if step == THROW then
            throwing(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("IronShl phase %#x reads past its phase table", step))
        end
    end

    return new.action { id = spec.id, state = throw.STATE, args = spec, update = update }
end

return throw
