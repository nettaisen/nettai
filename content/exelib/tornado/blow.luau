--!strict
-- Tornado's and Static's blow, as BN5 and BN6 share it (BN6's action 0x2F,
-- `sub_80ED454`; BN5's 0x22, 0x080ECD46), of a game's `TornadoBlowLook`
-- (its attachment kind and tornadoes' kind): the navi holds out a fan (an
-- attachment) and, ten ticks in, raises tornadoes (@exelib/tornado/tornado)
-- two panels ahead; the fan turns while they blow. Tornado raises one on
-- that panel; Static raises them on more panels the more kinds of NaviCust
-- bug the navi has (none: that panel; up to three: also the one behind it,
-- the two beside it, and the two beside that one). The action's third form
-- (its subtype 3) does the same by a side statistic (`sub_800AB3A`,
-- counter 3), spreading back toward the navi, with HP-bug tornadoes, and a
-- fan of another palette. 42 ticks, then 21 to put the fan away.

local attachment = require("../attachment")
local tornado = require("../tornado")

export type State = {
    -- Ticks left in the current phase (AIAttackVars+0x10).
    timer: number,
}

-- (What a chip's blow raises is a `TornadoBlow`.)
type Blow = TornadoBlow

-- One state for the action's blows.
local STATE = { timer = "u16" }

-- The phases, on the attack's step.
local BLOW, PUT_AWAY = 0x0, 0x4
-- The tick (timer value) the tornadoes rise.
local RISE_AT = 0x20

-- The panels each spread covers, by level (capped at 3): Static's
-- (`off_80CA160`) and the action's third form's (`off_80CA1F8`), { dx, dy }
-- from the panel two ahead, dx toward the navi's front.
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
-- The side statistic the third form spreads by.
local SIDE_STAT = 3

-- Turn the fan (its requested animation by `step`). Without one (the pool
-- was full) the game writes to address 0x10, which does nothing.
local function turn_fan(me: Object, step: number)
    local fan = me.overlay
    if fan then
        fan.anim = fan.anim + step
    end
end

-- A chip's blow, of a game's `look`.
local function blow(spec: Blow, look: TornadoBlowLook): Action
    local tornado_kind = spec.tornado_kind or look.tornado
    -- `sub_80CA104` / `sub_80CA19E`: the spread's tornadoes around (x, y).
    local function spread_tornadoes(me: Object, x: number, y: number, element: number, damage: number)
        local spread = spec.spread :: any
        local level = if spread.by == "bug_kinds"
            then battle.navi(me.alliance).bug_kinds
            else battle.side_stat(me.alliance, SIDE_STAT)
        local dir = me.facing
        level = math.min(level, 3)
        local tornadoes = spread.tornadoes :: { [number]: TornadoSpec }?
        local t: TornadoSpec = if tornadoes then tornadoes[level] else spread.tornado
        for _, p in spread.panels[level] do
            tornado.spawn(tornado_kind, me, x + dir * p[1], y + p[2], element, damage, t)
        end
    end

    -- `sub_80ED470`: hold out the fan; on the timer's 0x20 raise the
    -- tornadoes and turn the fan.
    local function blowing(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(0x0A)
            me:open_counter_window()
            attachment.attach(look.attachment, me, spec.fan, "overlay", { palette_add = spec.fan_palette })
            me:set_status("using_action", true)
            me.step_init = 4
            s.timer = 0x2A
        end
        if s.timer == RISE_AT then
            local x, y = me.panel_x + 2 * me.facing, me.panel_y
            -- The damage word (the hit parameter in its high half), plus
            -- the Atk+ bonus.
            local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
            local single = spec.single
            if single then
                tornado.spawn(tornado_kind, me, x, y, me.attack_element, damage, single)
            else
                spread_tornadoes(me, x, y, me.attack_element, damage)
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
    local function update(me: Object, s: State)
        local step = me.step
        if step == BLOW then
            blowing(me, s)
        elseif step == PUT_AWAY then
            put_away(me, s)
        else
            error(string.format("Tornado phase %#x reads past its phase table", step))
        end
    end

    return define.action { id = spec.id, state = STATE, args = spec, update = update }
end

local blows = {}

blows.STATE = STATE
blows.blow = blow
blows.STATIC_SPREAD, blows.BACK_SPREAD = STATIC_SPREAD, BACK_SPREAD

return blows
