--!strict
-- SloGauge's and FstGauge's dimming controller (the original's effect
-- object #0x1C, `sub_80E23A4`): the usual dimming phases (lib/dimming),
-- with an effect (`sub_80E23E8`) that makes the custom gauge fill slowly
-- or fast for the rest of the round, and sets the user's side's slow or
-- fast gauge timer (and, with per-player gauges outside a link battle,
-- the other side's), then shows a warning for 70 ticks.
--
-- The chips' `dimming` hooks (`sub_80E24B8`, `off_802CCB4[25]`) are
-- `gauge_speed.slow` and `gauge_speed.fast`: what the original's first
-- chip parameter told apart (0 SloGauge, 1 FstGauge).

local dimming_chips = require("@common/dimming")

local gauge_speed = {}

export type State = {
    -- FstGauge's fast gauge, or SloGauge's slow one.
    fast: boolean,
}

-- The gauge's fill rates, a tick.
local SLOW_RATE, FAST_RATE = 0x10, 0x40
-- The user's side's gauge timer, and the other side's.
local OWN_TICKS, OTHER_TICKS = 0x1E0, 0x438
local WAIT = 0x46
-- The warning's sound (`sub_800AE90`).
local WARNING_SOUND = asset.sound("bn6:err-select-91")

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
    local fast = (me.state :: State).fast
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
    -- sub_800AE90: a blinking warning over the gauge, with its sound. With
    -- per-player gauges in a link battle, only the user's player sees and
    -- hears it.
    if battle.per_player_gauges() and battle.link() then
        battle.warn(WARNING_SOUND, nil, me.alliance)
    else
        battle.warn(WARNING_SOUND)
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        dimming_chips.done(me)
    end
end

gauge_speed.kind = define.kind {
    id = "bn6:gauge-speed",
    pool = "effect",
    state = { fast = "bool" },
    -- `sub_80E23A4`.
    update = dimming_chips.phases { name = "a gauge chip", effect = effect },
}

-- A chip's `dimming` hook (`sub_80E24B8`): the controller for `user`, on
-- its side. (The spawner copies the user's side only, not its facing.)
local function hook(fast: boolean): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(gauge_speed.kind, user, spec, true)
        if o then
            local s = o.state :: State
            s.fast = fast
        end
        return o
    end
end

-- SloGauge's: the gauge fills slowly.
gauge_speed.slow = hook(false)
-- FstGauge's: the gauge fills fast.
gauge_speed.fast = hook(true)

return gauge_speed
