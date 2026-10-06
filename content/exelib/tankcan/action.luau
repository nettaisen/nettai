--!strict
-- The TankCans' action (EXE6's action 0x24, `sub_80ECACA`; EXE5's action 0x46, 0x080EEF4A, the same code but for the
-- cannon's attachment row): the navi aims its tank cannon for 16 ticks, then fires a shell (@exelib/tankcan/shell)
-- from the panel ahead on the 23rd, the screen shaking; it lowers the cannon on the 36th and is back to idle 4 ticks
-- later.

local attachment = require("../attachment")
local shell = require("./shell")

local tankcan = {}

export type State = {
    -- The phase's count (AIAttackVars+0x10).
    timer: number,
}

-- One state for the three.
tankcan.STATE = { timer = "u16" }

-- The phases, on the attack's step.
local AIM, FIRE, LOWER = 0x0, 0x4, 0x8

local ANIM_AIM, ANIM_FIRE, ANIM_LOWER = 0xA, 9, 7
-- The aim's ticks; the shell leaves on this tick of the count, and the phase ends on this one; the lowering's
-- ticks.
local AIM_TICKS, SHELL_TICK, FIRE_TICKS, LOWER_TICKS = 0xF, 5, 0x12, 3
local SHELL_Z = 0x18_0000
-- The cannon's recoil (its second animation).
local ARM_FIRE = 1

-- A chip's tank cannon: the cannon it holds (and its game's attachment kind), the shell kind and how its shell
-- flies.
export type Spec = TankCanSpec

function tankcan.action(spec: Spec): Action
    -- `sub_80ECAE8`: raise the cannon; aim for 16 ticks.
    local function aim(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(ANIM_AIM)
            me:open_counter_window()
            s.timer = AIM_TICKS
            attachment.attach(spec.attachment, me, spec.cannon, "overlay")
            me:set_status("using_action", true)
            me.step_init = 4
            return
        end
        local t = s.timer - 1
        s.timer = t
        if t <= 0 then
            me.step, me.step_init = FIRE, 0
        end
    end

    -- `sub_80ECB28`: the cannon recoils; the shell on the count's 5.
    local function fire(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(ANIM_FIRE)
            s.timer = 0
            local arm = me.overlay
            if arm then
                -- `sub_80B8E70`: restarted.
                arm.anim = ARM_FIRE
                arm.anim_loaded = 0xFF
            end
            me.step_init = 4
            return
        end
        if s.timer == SHELL_TICK then
            battle.shake_camera(1, 0x14)
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
            shell.fire(spec.kind, me, me.panel_x + me.facing, me.panel_y, SHELL_Z, damage, spec.shell)
        end
        local t = s.timer + 1
        s.timer = t
        if t == FIRE_TICKS then
            me.step, me.step_init = LOWER, 0
        end
    end

    -- `sub_80ECB84`: let go of the cannon, lower the arm, 4 ticks.
    local function lower(me: Object, s: State)
        if me.step_init == 0 then
            me.related1 = nil
            me.overlay = nil
            me:set_animation(ANIM_LOWER)
            s.timer = LOWER_TICKS
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            me:exit_attack()
        end
    end

    return new.action {
        id = spec.id,
        state = tankcan.STATE,
        args = spec,
        -- `sub_80ECACA`.
        update = function(me: Object, s: State)
            local step = me.step
            if step == AIM then
                aim(me, s)
            elseif step == FIRE then
                fire(me, s)
            elseif step == LOWER then
                lower(me, s)
            else
                error(string.format("TankCan phase %#x reads past off_80ECADC", step))
            end
        end,
    }
end

return tankcan
