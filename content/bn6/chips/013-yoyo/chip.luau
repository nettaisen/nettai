--!strict
-- YoYo and GreatYo: action 0x18 (`sub_80EC02A`). The navi holds out its
-- yoyo arm (an attachment) and throws a yoyo (objects/yoyo) from the panel
-- ahead, 27 pixels up; GreatYo (subtype 1) throws a controller
-- (objects/great-yoyo) that sends three, one on each row. The navi waits
-- until the yoyo (or the controller) clears the navi's first related
-- object, then puts the arm away over 6 ticks.

local attachment = require("../../objects/attachment/attachment")
local yoyo = require("../../objects/yoyo/yoyo")
local great_yoyo = require("../../objects/great-yoyo/great_yoyo")

export type State = {
    -- Ticks left in the current phase (AIAttackVars+0x10).
    timer: number,
}

local yoyo_chip = {
    state = { timer = "u16" },
}

-- The phases, on the attack's step.
local THROW, PUT_AWAY = 0x0, 0x4
-- The yoyo arm (attachment 0xE), and how high the yoyo flies.
local ARM = 0x0E
local HEIGHT = 0x1B_0000
-- GreatYo's subtype.
local GREAT = 1

-- `sub_80EC044`: throw; wait until what was thrown has gone.
local function throw(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0x0A)
        me:open_counter_window()
        attachment.spawn(me, { kind = ARM }, "overlay")
        me:set_status("using_action", true)
        local x, y = me.panel_x + me.facing, me.panel_y
        local params = { me:attack_param(1), me:attack_param(2), me:attack_param(3), me:attack_param(4) }
        -- The damage word (the hit parameter in its high half), plus the
        -- Atk+ bonus. The yoyos are null-element whatever the chip says.
        local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
        if me.variant == GREAT then
            me.related1 = great_yoyo.spawn(me, x, y, 0, HEIGHT, damage, params)
        else
            me.related1 = yoyo.spawn(me, x, y, 0, HEIGHT, damage, params, "related")
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
function yoyo_chip.update(me: Object, s: State)
    local step = me.step
    if step == THROW then
        throw(me, s)
    elseif step == PUT_AWAY then
        put_away(me, s)
    else
        error(string.format("YoYo phase %#x reads past its phase table", step))
    end
end

return yoyo_chip
