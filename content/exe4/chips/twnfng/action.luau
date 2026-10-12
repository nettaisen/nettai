--!strict
-- EXE4's twin fangs (the chips' action 0x34, 0x080ED4C6: EXE4's own). The navi (animation 0x12, a counter window)
-- sends pairs of fangs (./fang) from its own panel, one a row down and one a row up, then each forward along its row,
-- 10 ticks apart: the variant times 7, and one (0x080ED4FE: TwnFng's 1, BstFang's 8); their hits strong or weak (the
-- parameters' second byte: TwnFng's 0, BstFang's 1); then 31 ticks, and back to idle.

local fang = require("./fang")

local twnfng = {}

local ANIM = 0x12
local INTERVAL, RECOVER_TICKS = 0x0A, 0x1E

type State = {
    -- The countdown to the next pair (AIAttackVars +0x10) and the pairs left (+0x12).
    timer: number,
    pairs: number,
}

-- A chip's fangs: its key, how many pairs and whether their hits are weak.
function twnfng.action(id: string, pairs: number, weak: boolean): Action
    -- 0x080ED4E8: the pose; the first pair at once.
    local function start(me: Object, s: State)
        me:set_animation(ANIM)
        me:open_counter_window()
        me:set_status("using_action", true)
        s.pairs = pairs
        s.timer = 0
        me.step, me.step_init = 4, 0
    end

    -- 0x080ED512: a pair of fangs when the countdown ends, 10 ticks apart; after the last, the recovery.
    local function send(me: Object, s: State)
        local t = s.timer - 1
        s.timer = int.u16(t)
        if t > 0 then
            return
        end
        s.timer = INTERVAL
        local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
        fang.spawn(me, me.panel_x, me.panel_y, me.attack_element, damage, false, weak)
        fang.spawn(me, me.panel_x, me.panel_y, me.attack_element, damage, true, weak)
        local left = s.pairs - 1
        s.pairs = int.u16(left)
        if left <= 0 then
            me.step, me.step_init = 8, 0
        end
    end

    -- 0x080ED554: 31 ticks, then back to idle.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            s.timer = RECOVER_TICKS
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = int.u16(t)
        if t < 0 then
            me:exit_attack()
        end
    end

    return new.action {
        id = id,
        state = { timer = "u16", pairs = "u16" },
        -- 0x080ED4C6.
        update = function(me: Object, s: State)
            local step = me.step
            if step == 0 then
                start(me, s)
            elseif step == 4 then
                send(me, s)
            elseif step == 8 then
                recover(me, s)
            else
                error(string.format("TwnFng's phase %#x reads past its table (0x080ED4DC)", step))
            end
        end,
    }
end

return twnfng
