--!strict
-- The SloGauge and FstGauge chips' dimming controller (effect object #0x1C,
-- `sub_80E23A4`): the usual dimming phases (the dimming service), with an
-- effect (`sub_80E23E8`) that makes the custom gauge fill slowly (Param1 0)
-- or fast (Param1 1) for the rest of the round, and sets the user's side's
-- slow or fast gauge timer (and, with per-player gauges outside a link
-- battle, the other side's), then shows a warning for 70 ticks.
--
-- The chips' script (both are dimming chip subtype 25): `dimming_chip` is
-- their controller's spawner, `off_802CCB4[25]`.

export type State = {
    -- The chip and the Atk+ / cross bonus the telop shows (object +0x30,
    -- +0x32).
    chip: number,
    bonus: number,
}

local gauge_speed = {
    state = { chip = "u16", bonus = "u16" },
}

-- The gauge's fill rates, a tick.
local SLOW_RATE, FAST_RATE = 0x10, 0x40
-- The user's side's gauge timer, and the other side's.
local OWN_TICKS, OTHER_TICKS = 0x1E0, 0x438
local WAIT = 0x46
-- The warning's sound (`sub_800AE90`).
local WARNING_SOUND = 0x91

-- `sub_80E24B8`: the controller for `user`, on its panel, with the chip's
-- parameters and damage word. (Its position is the spawner's registers,
-- which nothing reads.)
function gauge_speed.dimming_chip(user: Object, spec: DimmingChipSpec): Object?
    local o = battle.spawn_kind("gauge-speed", Vec3.zero, spec.params)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = user.panel_x, user.panel_y
    o.element = spec.element
    o.related1 = user
    o.alliance = user.alliance
    o.damage = spec.damage
    o.stamina = bit32.rshift(spec.damage, 16)
    local s = o.state :: State
    s.chip, s.bonus = spec.chip, spec.bonus
    return o
end

-- A side's timers: the slow one for SloGauge, the fast one for FstGauge.
local function set_timer(side: number, fast: boolean, ticks: number)
    if fast then
        battle.set_gauge_speed_ticks(side, 0, ticks)
    else
        battle.set_gauge_speed_ticks(side, ticks, 0)
    end
end

-- `sub_80E23E8`.
local function effect(me: Object)
    local fast = me:param(1) ~= 0
    if me.phase_init == 0 then
        -- sub_801DF8C
        battle.set_gauge_rate(if fast then FAST_RATE else SLOW_RATE)
        set_timer(me.alliance, fast, OWN_TICKS)
        if battle.per_player_gauges() and not battle.link() then
            set_timer(bit32.bxor(me.alliance, 1), fast, OTHER_TICKS)
        end
        -- (The gauge's slow or fast look is loaded into VRAM.)
        me.timer = WAIT
        me.phase_init = 4
    end
    -- sub_800AE90: a blinking warning over the gauge (drawn only), with
    -- its sound every 16 frames of the game's frame counter, which the
    -- port doesn't keep: every 16 ticks of this effect instead. With
    -- per-player gauges in a link battle, only the user's player sees and
    -- hears it.
    if me.timer % 16 == 0 then
        if battle.per_player_gauges() and battle.link() then
            battle.play_sound_for(me.alliance, WARNING_SOUND)
        else
            battle.play_sound(WARNING_SOUND)
        end
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        me:set_action(0xC)
    end
end

-- `sub_80E23A4`: the dimming's phases around the effect.
function gauge_speed.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        dimming.begin(me)
    elseif l == "update" then
        local a = me.action
        if a == 0 then
            dimming.dim_screen(me)
        elseif a == 4 then
            dimming.show_telop(me)
        elseif a == 8 then
            effect(me)
        elseif a == 0xC then
            dimming.undim_screen(me)
        else
            error(string.format("the gauge chips' action %#x reads past their table", a))
        end
    else
        dimming.finish(me)
    end
end

return gauge_speed
