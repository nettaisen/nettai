--!strict
-- Tornado and Static: action 0x2F (`sub_80ED454`). The navi holds out a
-- fan (an attachment) and, ten ticks in, raises tornadoes (attack object
-- #0x31, objects/tornado) two panels ahead; the fan turns while they blow.
-- By the chip's subtype:
--
--   1 Tornado: one tornado on that panel, with the chip's parameters.
--   2 Static: tornadoes on more panels the more kinds of NaviCust bug the
--     navi has (none: that panel; up to three: also the one behind it, the
--     two beside it, and the two beside that one), a static tornado each.
--   3 (no chip): the same by a side statistic (`sub_800AB3A`, counter 3),
--     spreading back toward the navi, with a HP-bug tornado each.
--
-- 42 ticks, then 21 to put the fan away.

local attachment = require("../../objects/attachment/attachment")
local tornado = require("../../objects/tornado/tornado")

export type State = {
    -- Ticks left in the current phase (AIAttackVars+0x10).
    timer: number,
}

local tornado_chip = {
    state = { timer = "u16" },
}

-- The phases, on the attack's step.
local BLOW, PUT_AWAY = 0x0, 0x4
-- The tick (timer value) the tornadoes rise.
local RISE_AT = 0x20
-- The fan: attachment 0x21, or 0x22 for subtypes 2 and 3 (with palette 1
-- for 3).
local FAN, CHARGED_FAN = 0x21, 0x22

-- The panels each spread covers, by level (capped at 3): Static's
-- (`off_80CA160`) and subtype 3's (`off_80CA1F8`), { dx, dy } from the
-- panel two ahead, dx toward the navi's front.
local STATIC_SPREAD = {
    [0] = { { 0, 0 } },
    { { 0, 0 }, { 1, 0 } },
    { { 0, 0 }, { 1, 0 }, { 0, -1 }, { 0, 1 } },
    { { 0, 0 }, { 1, 0 }, { 0, -1 }, { 0, 1 }, { 1, -1 }, { 1, 1 } },
}
local BACK_SPREAD = {
    [0] = { { 0, 0 } },
    { { 0, 0 }, { -1, 0 } },
    { { 0, 0 }, { -1, 0 }, { 0, -1 }, { 0, 1 } },
    { { 0, 0 }, { -1, 0 }, { 0, -1 }, { 0, 1 }, { 1, 0 } },
}
-- The tornado parameters of each spread (`byte_80CA170`, `byte_80CA208`):
-- kind 2 or 3, 0x20 ticks.
local STATIC_PARAMS = { 2, 0, 0x20, 0 }
local BACK_PARAMS = { 3, 0, 0x20, 0 }

-- `sub_80CA104` / `sub_80CA19E`: a tornado on each panel of `spread` at
-- `level` around (x, y).
local function spread_tornadoes(
    me: Object,
    x: number,
    y: number,
    element: number,
    damage: number,
    spread: { [number]: { { number } } },
    level: number,
    params: { number }
)
    local dir = me.facing
    for _, p in spread[math.min(level, 3)] do
        tornado.spawn(me, x + dir * p[1], y + p[2], element, damage, params)
    end
end

-- Turn the fan (its requested animation by `step`). Without one (the pool
-- was full) the game writes to address 0x10, which does nothing.
local function turn_fan(me: Object, step: number)
    local fan = me.overlay
    if fan then
        fan.anim = fan.anim + step
    end
end

-- `sub_80ED470`: hold out the fan; on the timer's 0x20 raise the
-- tornadoes and turn the fan.
local function blow(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0x0A)
        me:open_counter_window()
        local variant = me.variant
        local kind, palette = FAN, 0
        if variant == 2 then
            kind = CHARGED_FAN
        elseif variant == 3 then
            kind, palette = CHARGED_FAN, 1
        end
        attachment.spawn(me, { kind = kind, palette_add = palette }, "overlay")
        me:set_status("using_action", true)
        me.step_init = 4
        s.timer = 0x2A
    end
    if s.timer == RISE_AT then
        local x, y = me.panel_x + 2 * me.facing, me.panel_y
        -- The damage word (the hit parameter in its high half), plus the
        -- Atk+ bonus.
        local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
        local element = me.attack_element
        local variant = me.variant
        if variant == 2 then
            local level = battle.navi(me.alliance).bug_kinds
            spread_tornadoes(me, x, y, element, damage, STATIC_SPREAD, level, STATIC_PARAMS)
        elseif variant == 3 then
            local level = battle.side_stat(me.alliance, 3)
            spread_tornadoes(me, x, y, element, damage, BACK_SPREAD, level, BACK_PARAMS)
        else
            local params = { me:attack_param(1), me:attack_param(2), me:attack_param(3), me:attack_param(4) }
            tornado.spawn(me, x, y, element, damage, params)
        end
        turn_fan(me, 1)
    end
    local t = s.timer - 1
    s.timer = t
    if t <= 0 then
        me.step = PUT_AWAY
        me.step_init = 0
    end
end

-- `sub_80ED52C`: turn the fan back; 21 ticks later, back to idle.
local function put_away(me: Object, s: State)
    if me.step_init == 0 then
        turn_fan(me, -1)
        s.timer = 0x14
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

-- `sub_80ED454`.
function tornado_chip.update(me: Object, s: State)
    local step = me.step
    if step == BLOW then
        blow(me, s)
    elseif step == PUT_AWAY then
        put_away(me, s)
    else
        error(string.format("Tornado phase %#x reads past its phase table", step))
    end
end

return tornado_chip
