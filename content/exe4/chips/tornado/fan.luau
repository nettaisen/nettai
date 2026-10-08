--!strict
-- The fan's action (action 0x28, 0x080ECF44: EXE4's own; Tornado's and Static's, chips/tornado, and WoodSoul's charged
-- shot): the navi (animation 13, a counter window) holds up the fan (an attachment row, its animation twice the navi's
-- number); 10 ticks in, the fan's animation one on and the user's blow two panels ahead; 42 ticks; then the fan's
-- animation one back, 21 ticks, and back to idle without it. Each user makes its own (`fan.action`), stating its fan's
-- look (the variant's attachment row: 2's 0x26, the others' 0x0F) and its blow (variant 2's static tornadoes,
-- 0x080D3152; the others' one tornado, 0x080D3130, of the attack's parameters).

local attachment = require("../../objects/attachment")

local fan = {}

-- The navi's number: what it holds animates by it (NaviStats +0x23, MegaMan's 0).
local NAVI_NUMBER = 0

local ANIM = 13
local HOLD_TICKS, BLOW_AT, RECOVER_TICKS = 42, 32, 20
local AHEAD = 2

type State = { timer: number }

-- A blow: the tornadoes on (x, y) and around, with the attack's element and damage word.
export type FanBlow = (me: Object, x: number, y: number, element: number, damage: number) -> ()

-- The fan's action `id`, holding up `look` and blowing `blow`.
function fan.action(id: string, look: AttachmentLook, blow: FanBlow): Action
    -- 0x080ECF64: the fan up; 10 ticks in, the tornadoes and the fan's next animation; 42 ticks.
    local function hold(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(ANIM)
            me:open_counter_window()
            attachment.attach(me, look, "related", 2 * NAVI_NUMBER)
            me:set_status("using_action", true)
            me.step_init = 4
            s.timer = HOLD_TICKS
        end
        if s.timer == BLOW_AT then
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
            blow(me, me.panel_x + AHEAD * me.facing, me.panel_y, me.attack_element, damage)
            local f = me.related1
            if f then
                f.anim += 1
            end
        end
        local t = s.timer - 1
        s.timer = int.u16(t)
        if t <= 0 then
            me.step, me.step_init = 4, 0
        end
    end

    -- 0x080ED000: the fan's animation back; 21 ticks, then back to idle without it.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            local f = me.related1
            if f then
                f.anim -= 1
            end
            s.timer = RECOVER_TICKS
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = int.u16(t)
        if t < 0 then
            me.related1 = nil
            me:exit_attack()
        end
    end

    return new.action {
        id = id,
        state = { timer = "u16" },
        -- 0x080ECF44.
        update = function(me: Object, s: State)
            local step = me.step
            if step == 0 then
                hold(me, s)
            elseif step == 4 then
                recover(me, s)
            else
                error(string.format("a fan's phase %#x reads past its table (0x080ECF5C)", step))
            end
        end,
    }
end

return fan
