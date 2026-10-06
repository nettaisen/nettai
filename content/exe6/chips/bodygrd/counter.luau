--!strict
-- BodyGrd's counter (the original's navi action 0x4B, `sub_80EED56`): when
-- a BodyGrd trap catches a hit, the navi vanishes in a puff and leaves a
-- striker (./striker) that drops shurikens on the enemy navi. The rules
-- starts it by role (`actions.body_guard_counter`, rules/roles.luau).

local effects = require("../../lib/effects")
local regions = require("@exelib/regions")
local striker = require("./striker")

local counter = {}

export type State = {
    -- Ticks left in the phase (AIAttackVars+0x10).
    timer: number,
}

local STATE = { timer = "u16" }

local SOUND_VANISH = asset.sound("trap-vanish")
local VANISH_ANIM = 4

local function set_step(me: Object, step: number)
    me.step, me.step_init = step, 0
end

-- `sub_80EED74`: stand still on the panel (reserved), uncollidable; the
-- side's trap record goes; 4 ticks later, a puff, and on.
local function vanish(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(VANISH_ANIM)
        me:unreserve_panel(me.future_panel_x, me.future_panel_y)
        me:set_status("sliding", false)
        me:set_status("moving", true)
        me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
        me:reserve_panel(me.panel_x, me.panel_y)
        me.collision:set_region(nil)
        battle.clear_linked(me.alliance)
        s.timer = 3
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t >= 0 then
        return
    end
    local p = me.pos
    battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), effects.bubbles)
    battle.play_sound(SOUND_VANISH)
    set_step(me, 4)
end

-- `sub_80EEDDE`: hidden, leave the striker; 31 ticks.
local function strike(me: Object, s: State)
    me.visible = false
    if me.step_init == 0 then
        local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
        striker.spawn(me, me.attack_element, damage)
        s.timer = 0x1E
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t < 0 then
        set_step(me, 8)
    end
end

-- `sub_80EEE10`: back, its trap request done; 6 ticks, then idle.
local function back(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0)
        me.collision:set_region(regions.single)
        me:unreserve_panel(me.future_panel_x, me.future_panel_y)
        me:set_request("body_guard_triggered", false)
        me:set_status("moving", false)
        s.timer = 5
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t < 0 then
        me:exit_attack()
    end
end

counter.action = new.action {
    id = "bodygrd/counter",
    state = STATE,
    -- `sub_80EED56`, by phase.
    update = function(me: Object, s: State)
        local step = me.step
        if step == 0 then
            vanish(me, s)
        elseif step == 4 then
            strike(me, s)
        elseif step == 8 then
            back(me, s)
        else
            error(string.format("BodyGrd's counter: phase %#x reads past off_80EED68", step))
        end
    end,
}

return counter
