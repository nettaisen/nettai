--!strict
-- The yoyo chips' throw (BN6's action 0x18, `sub_80EC02A`; BN5's action 0x4A, 0x080EF1E4, the same code but for the
-- arm's attachment row): the navi holds out its yoyo arm (an attachment) and throws a yoyo (or GreatYo's
-- controller) from the panel ahead, 27 pixels up, null-element. The navi waits until what it threw clears the
-- navi's first related object, then puts the arm away over 6 ticks.
--
-- Each game's chips throw their own yoyos: BN6's chips/yoyo, BN5's objects/yoyo.

local attachment = require("../attachment/attachment")

local throw = {}

export type State = {
    -- Ticks left in the current phase (AIAttackVars+0x10).
    timer: number,
}

-- One state for every throw.
throw.STATE = { timer = "u16" }

-- A chip's throw: the action's key, the yoyo arm (and its game's attachment kind), and what it throws (from (x, y),
-- `HEIGHT` up, with the damage word: the yoyo or the controller, which the navi holds in its first related object).
export type Spec = YoyoThrowSpec

-- The phases, on the attack's step.
local THROW, PUT_AWAY = 0x0, 0x4
-- How high the yoyo flies.
local HEIGHT = 0x1B_0000

function throw.action(spec: Spec): Action
    -- `sub_80EC044`: throw; wait until what was thrown has gone.
    local function throwing(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(0x0A)
            me:open_counter_window()
            attachment.attach(spec.attachment, me, spec.arm, "overlay")
            me:set_status("using_action", true)
            local x, y = me.panel_x + me.facing, me.panel_y
            -- The damage word (the hit parameter in its high half), plus the Atk+ bonus. The yoyos are
            -- null-element whatever the chip says.
            local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
            me.related1 = spec.throw(me, x, y, HEIGHT, damage)
            s.timer = 0x1E
            me.step_init = 4
        end
        if me.related1 == nil then
            me.step = PUT_AWAY
            me.step_init = 0
        end
    end

    -- `sub_80EC0C0`: 6 ticks, then back to idle.
    local function put_away(me: Object, s: State)
        if me.step_init == 0 then
            s.timer = 5
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = t
        if t >= 0 then
            return
        end
        me.related1 = nil
        me.overlay = nil
        me:exit_attack()
    end

    -- `sub_80EC02A`.
    local function update(me: Object, s: State)
        local step = me.step
        if step == THROW then
            throwing(me, s)
        elseif step == PUT_AWAY then
            put_away(me, s)
        else
            error(string.format("YoYo phase %#x reads past its phase table", step))
        end
    end

    return define.action { id = spec.id, state = throw.STATE, args = spec, update = update }
end

return throw
