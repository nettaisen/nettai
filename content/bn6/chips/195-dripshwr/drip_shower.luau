--!strict
-- SpoutMan's own chip, DripShwr (action 0x0A for AI index 6,
-- `sub_80F15CE`): he winds up (35 ticks), then jumps to the panel three
-- ahead if it is solid and free, sprays water around himself there (attack
-- object #0x29, objects/drip-shower: two turns of four quarters, 10 ticks
-- each), and jumps back. If he can't move or the panel won't do, he only
-- lands again. [unverified: no lab scenario reaches it;
-- docs/engine/standard-chips.md]

local drip_shower = require("../../objects/drip-shower/drip_shower")
local link = require("../190-heatpres/state")
type State = LinkChipState

local routine = {}

local WIND_UP, JUMP, FLY, LAND, SPRAY, RISE, RETURN = 0x0, 0x4, 0x8, 0xC, 0x10, 0x14, 0x18
-- The panel he jumps to: solid, with nothing on it or reserved.
local SOLID, OCCUPIED = 0x10, 0x0F880080
-- The spray: the link chip's kind (4), two turns, 10 ticks a quarter.
local SPRAY_PARAMS = { 4, 2, 0x0A }

-- `sub_80F15FC`: animation 0x11, 0x12 after 5 ticks; 35 ticks.
local function wind_up(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = 0x11
        s.timer = 0x23
        return
    end
    local t = s.timer - 1
    s.timer = t
    if t == 0x1E then
        me.anim = 0x12
    elseif t <= 0 then
        link.go(me, JUMP)
    end
end

-- `sub_80F162C`: to the panel three ahead, if he can go (reserving it and
-- his own); else he lands where he is.
local function jump(me: Object, s: State)
    if me:can_move() then
        local x, y = me.panel_x + me.facing * 3, me.panel_y
        if field.check(x, y, SOLID, OCCUPIED) then
            s.target_x, s.target_y = x, y
            me:reserve_panel(x, y)
            me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
            me:reserve_panel(me.panel_x, me.panel_y)
            me.collision.region = 0
            me:set_status("moving", true)
            s.timer = 2
            link.go(me, FLY)
            return
        end
    end
    s.timer = 2
    link.go(me, LAND)
end

-- `sub_80F1694`: two ticks, then onto the target panel.
local function fly(me: Object, s: State)
    if not link.count_down(s) then
        return
    end
    me.panel_x, me.panel_y = s.target_x, s.target_y
    me:set_coordinates_from_panel()
    me:update_collision_panels()
    link.go(me, SPRAY)
end

-- `sub_80F16B4`: two ticks, animation 3 for 8, then idle.
local function land(me: Object, s: State)
    if me.step_init == 0 then
        if link.count_down(s) then
            me.anim = 3
            s.timer = 8
            me.step_init = 4
        end
    elseif link.count_down(s) then
        me:exit_attack()
    end
end

-- `sub_80F16E0`: the spray on his panel; 90 ticks (its two turns).
local function spray(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        link.add_bonus(me)
        drip_shower.spawn(me, me.panel_x, me.panel_y, me.attack_element, SPRAY_PARAMS, link.damage_word(me), 0x0A_0000)
        s.timer = 0x0A * 4 * 2 + 0x0A
    elseif link.count_down(s) then
        link.go(me, RISE)
    end
end

-- `sub_80F172C`: animation 4, moving, 6 ticks.
local function rise(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = 4
        me:set_status("moving", true)
        s.timer = 6
    elseif link.count_down(s) then
        link.go(me, RETURN)
    end
end

-- `sub_80F1756`: back on his own panel (both reservations go), region 1,
-- the move done; 8 ticks, then idle.
local function return_home(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = 3
        me:unreserve_panel(me.panel_x, me.panel_y)
        me.panel_x, me.panel_y = me.future_panel_x, me.future_panel_y
        me:unreserve_panel(me.panel_x, me.panel_y)
        me:set_coordinates_from_panel()
        me:update_collision_panels()
        me.collision.region = 1
        me:set_status("moving", false)
        me:set_status("move_complete", true)
        me:set_status("using_action", false)
        s.timer = 8
    elseif link.count_down(s) then
        me:exit_attack()
    end
end

-- `sub_80F15CE` (`off_80F15E0`).
function routine.update(me: Object, s: State)
    local step = me.step
    if step == WIND_UP then
        wind_up(me, s)
    elseif step == JUMP then
        jump(me, s)
    elseif step == FLY then
        fly(me, s)
    elseif step == LAND then
        land(me, s)
    elseif step == SPRAY then
        spray(me, s)
    elseif step == RISE then
        rise(me, s)
    elseif step == RETURN then
        return_home(me, s)
    else
        error(string.format("DripShwr phase %#x reads past off_80F15E0", step))
    end
end

return routine
