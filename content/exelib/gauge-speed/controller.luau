--!strict
-- SloGauge's and FstGauge's dimming controller, as BN5 and BN6 share it
-- (BN6's effect object #0x1C, `sub_80E23A4`; BN5's #0x1C): each game makes
-- its own (`gauge_speed.make`: its kind and its warning's sound). The usual
-- dimming phases (@exelib/dimming),
-- with an effect (`sub_80E23E8`) that makes the custom gauge fill slowly
-- or fast for the rest of the round, and sets the user's side's slow or
-- fast gauge timer (and, in the own-gauges mode outside a link battle,
-- the other side's), then shows a warning for 70 ticks.
--
-- The chips' `dimming` hooks (`sub_80E24B8`, `off_802CCB4[25]`) are the
-- made controller's `slow` and `fast`: what the original's first chip
-- parameter told apart (0 SloGauge, 1 FstGauge).

local dimming_chips = require("../dimming")

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

-- A game's controller: its kind's key and the warning's sound
-- (`sub_800AE90`).
export type Spec = { id: string, warning: Sound }

export type GaugeSpeed = {
    kind: Kind,
    -- SloGauge's hook: the gauge fills slowly.
    slow: (user: Object, spec: DimmingChipSpec) -> Object?,
    -- FstGauge's hook: the gauge fills fast.
    fast: (user: Object, spec: DimmingChipSpec) -> Object?,
}

-- A side's timers: the slow one for SloGauge, the fast one for FstGauge.
local function set_timer(side: number, fast: boolean, ticks: number)
    if fast then
        battle.set_gauge_speed_ticks(side, 0, ticks)
    else
        battle.set_gauge_speed_ticks(side, ticks, 0)
    end
end

-- `sub_80E23E8`.
local function effect(me: Object, WARNING_SOUND: Sound)
    local fast = (me.state :: State).fast
    if me.phase_init == 0 then
        -- sub_801DF8C
        battle.set_gauge_rate(if fast then FAST_RATE else SLOW_RATE)
        set_timer(me.alliance, fast, OWN_TICKS)
        if battle.own_gauges() and not battle.link() then
            set_timer(bit32.bxor(me.alliance, 1), fast, OTHER_TICKS)
        end
        -- (The gauge's slow or fast look is loaded into VRAM.)
        me.timer = WAIT
        me.phase_init = 4
    end
    -- sub_800AE90: a blinking warning over the gauge, with its sound. With
    -- each side its own gauge in a link battle, only the user's player sees and
    -- hears it.
    if battle.own_gauges() and battle.link() then
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

-- A game's controller.
function gauge_speed.make(spec: Spec): GaugeSpeed
    local kind = define.kind {
        id = spec.id,
        pool = "effect",
        state = { fast = "bool" },
        -- `sub_80E23A4`.
        update = dimming_chips.phases {
            name = "a gauge chip",
            effect = function(me: Object)
                effect(me, spec.warning)
            end,
        },
    }

    -- A chip's `dimming` hook (`sub_80E24B8`): the controller for `user`, on
    -- its side. (The spawner copies the user's side only, not its facing.)
    local function hook(fast: boolean): (user: Object, spec: DimmingChipSpec) -> Object?
        return function(user: Object, chip: DimmingChipSpec): Object?
            local o = dimming_chips.spawn(kind, user, chip, true)
            if o then
                local s = o.state :: State
                s.fast = fast
            end
            return o
        end
    end

    return { kind = kind, slow = hook(false), fast = hook(true) }
end

return gauge_speed
