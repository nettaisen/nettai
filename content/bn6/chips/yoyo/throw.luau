--!strict
-- YoYo's and GreatYo's throw (the original's action 0x18, `sub_80EC02A`):
-- the navi holds out its yoyo arm (an attachment) and throws a yoyo
-- (yoyo.luau) from the panel ahead, 27 pixels up; GreatYo throws a
-- controller that sends three, one on each row. The navi waits until the
-- yoyo (or the controller) clears the navi's first related object, then
-- puts the arm away over 6 ticks.

local attachment = require("../../objects/attachment/attachment")
local yoyo = require("./yoyo")

local throw = {}

export type State = {
    -- Ticks left in the current phase (AIAttackVars+0x10).
    timer: number,
}

-- One state for both throws.
throw.STATE = { timer = "u16" }

-- The phases, on the attack's step.
local THROW, PUT_AWAY = 0x0, 0x4
-- The yoyo arm (the original's attachment row 0x0E), and how high the yoyo
-- flies.
local ARM = attachment.look { sprite = asset.sprite("bn6:yoyo-arm"), palette = 0, attach_point = 0x0F }
local HEIGHT = 0x1B_0000

-- A chip's throw: how its yoyos fly (a GreatYo's throws the controller);
-- `id`, the action's key (its compat key: both chips are records).
function throw.action(spec: { id: string, yoyo: YoyoSpec }): Action
    -- `sub_80EC044`: throw; wait until what was thrown has gone.
    local function throwing(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(0x0A)
            me:open_counter_window()
            attachment.attach(me, ARM, "overlay")
            me:set_status("using_action", true)
            local x, y = me.panel_x + me.facing, me.panel_y
            -- The damage word (the hit parameter in its high half), plus
            -- the Atk+ bonus. The yoyos are null-element whatever the chip
            -- says.
            local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
            if spec.yoyo.great then
                me.related1 = yoyo.spawn_controller(me, x, y, 0, HEIGHT, damage, spec.yoyo)
            else
                me.related1 = yoyo.spawn(me, x, y, 0, HEIGHT, damage, spec.yoyo, "related")
            end
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
