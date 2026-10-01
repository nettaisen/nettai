--!strict
-- The TimeBom chips' dimming controller (the original's effect object
-- #0x27, `sub_80E31D8`): the usual dimming phases (lib/dimming), with an
-- effect that sets the chip's countdown bomb (./countdown) on the first
-- free enemy panel in front of the user, and waits 31 ticks.
--
-- A chip's `dimming` hook (`sub_80E3242`, `off_802CCB4[10]`) is
-- `controller.hook(variant)`: the bomb the original's first chip
-- parameter picked (0 TimeBom1-3's, 1 TimeBom+'s) is the hook's argument.

local countdown = require("./countdown")
local dimming_chips = require("../../lib/dimming")

export type State = {
    -- The Atk+ / cross bonus the telop shows (object +0x32), which adds to
    -- the bomb's damage.
    bonus: number,
    -- The bomb the chip sets.
    variant: CountdownVariant,
}

local WAIT = 0x1E

-- `off_80E3280`: where the bomb goes, by side: a solid enemy panel without
-- a body on it (side 0: flags 0x30, not 0x0F800000; side 1: 0x10, not
-- 0x0F800020).
local TARGET: { [number]: PanelCondition } = {
    [0] = { require = 0x30, forbid = 0x0F80_0000 },
    [1] = { require = 0x10, forbid = 0x0F80_0020 },
}

-- `sub_80E3264` (`object_getFirstPanelInDirectionFiltered`): from the
-- controller's panel forward, the first panel meeting the side's
-- condition, while the panels are on the field (nil: none).
local function target_column(me: Object): number?
    local cond = TARGET[me.alliance]
    local x, y = me.panel_x, me.panel_y
    while true do
        if field.check(x, y, cond.require, cond.forbid) then
            return x
        end
        x += me.facing
        if not field.valid(x, y) then
            return nil
        end
    end
end

-- `sub_80E3290`: the bomb, with the damage plus the bonus.
local function place(me: Object)
    local x = target_column(me)
    -- (The game tests the column number for zero: column 0 is off the
    -- field, so a found panel always places.)
    if x then
        local s = me.state :: State
        countdown.spawn(s.variant, x, me.panel_y, me.alliance, dimming_chips.damage_plus(me, s.bonus), s.bonus)
    end
end

-- `sub_80E321C`: the bomb, then 31 ticks.
local function effect(me: Object)
    if me.phase_init == 0 then
        me.timer = WAIT
        place(me)
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        dimming_chips.done(me)
    end
end

local kind = define.kind {
    id = "timebom/controller",
    pool = "effect",
    state = { bonus = "u16", variant = "record:countdown-variant" },
    -- `sub_80E31D8`.
    update = dimming_chips.phases { name = "TimeBom", effect = effect },
}

-- A chip's `dimming` hook (`sub_80E3242`): the controller for `user`,
-- setting a bomb of `variant`.
local function hook(variant: CountdownVariant): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(kind, user, spec)
        if o then
            local s = o.state :: State
            s.bonus = spec.bonus
            s.variant = variant
        end
        return o
    end
end

return { kind = kind, hook = hook }
