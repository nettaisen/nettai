--!strict
-- RskyHny1/2/3: action 0x39 (`sub_80EDD80`). See chips.md §3.7.
--
-- MegaMan holds up a hive and a bee flies out ahead (objects/honey-bee).
-- For three windows of 11 ticks after that, a hit that isn't fire (the
-- navi's RskyHny trap, `sub_802CEF4`, sets the action's marker) sends
-- another bee, once a window. The chip's level (its first parameter) is
-- the bees' speed.

local attachment = require("../../objects/attachment/attachment")
local honey_bee = require("../../objects/honey-bee/honey_bee")

export type State = {
    -- Ticks left in the current phase or window (+0x10).
    timer: number,
    -- Windows left (+0x12).
    windows: number,
}

local rsky_hny = {
    state = { timer = "u16", windows = "u16" },
}

-- The phases, on the attack's step.
local RAISE, RELEASE, WINDOWS = 0x0, 0x4, 0x8

-- The hive (an attachment), and the sound of a bee.
local HIVE = 0x28
local BUZZ = 0x1A8
local WINDOW_TICKS, WINDOW_COUNT = 10, 3

local function set_phase(me: Object, step: number)
    me.step = step
    me.step_init = 0
end

-- `sub_80EDE4A`: a bee on the panel ahead, with the attack's parameters
-- and damage word (bonus included); the hive opens.
local function release_bee(me: Object)
    local params = { me:attack_param(1), me:attack_param(2), me:attack_param(3), me:attack_param(4) }
    local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
    honey_bee.spawn(me, me.panel_x + me.facing, me.panel_y, me.attack_element, params, damage)
    local hive = me.overlay
    if hive then
        hive.anim = 1
        hive.anim_loaded = 0xFF
    end
    battle.play_sound(BUZZ)
end

-- `sub_80EDDA0`: raise the hive and arm the trap; two ticks later, on.
local function raise(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0x0A)
        attachment.spawn(me, { kind = HIVE }, "overlay")
        me:set_status("using_action", true)
        me:set_navi_state("heat_trap", true)
        me.step_init = 4
        s.timer = 1
    end
    local t = s.timer - 1
    s.timer = t
    if t < 0 then
        set_phase(me, RELEASE)
    end
end

-- `sub_80EDDE0`: the first bee. (With a second parameter, which no chip
-- has, the game instead clears the navi's drag step.)
local function release(me: Object, s: State)
    if me:attack_param(2) ~= 0 then
        me.drag_step = "start"
    else
        release_bee(me)
    end
    s.windows = WINDOW_COUNT
    set_phase(me, WINDOWS)
end

-- `sub_80EDDFC`: a window of 11 ticks, in which a trapped hit sends a
-- bee; after the last window, put the hive away and disarm the trap.
local function windows(me: Object, s: State)
    local init = me.step_init
    if init == 0 then
        me.step_init = 1
        s.timer = WINDOW_TICKS
        me.marker = 0
        init = 1
    end
    if init == 1 and me.marker ~= 0 then
        release_bee(me)
        me.step_init = 2
    end
    local t = s.timer - 1
    s.timer = t
    if t >= 0 then
        return
    end
    local left = s.windows - 1
    s.windows = left
    if left > 0 then
        me.step_init = 0
        return
    end
    me.related1 = nil
    me.overlay = nil
    me:set_navi_state("heat_trap", false)
    me:exit_attack()
end

-- `sub_80EDD80`.
function rsky_hny.update(me: Object, s: State)
    local step = me.step
    if step == RAISE then
        raise(me, s)
    elseif step == RELEASE then
        release(me, s)
    elseif step == WINDOWS then
        windows(me, s)
    else
        error(string.format("RskyHny phase %#x reads past its phase table", step))
    end
end

return rsky_hny
