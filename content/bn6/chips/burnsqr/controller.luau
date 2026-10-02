--!strict
-- BurnSqr1-3's dimming controller (the original's effect object #0x59,
-- `sub_80E702C`; docs/engine/dimming-chip-effects.md §2): the usual dimming
-- phases (lib/dimming), with an effect that sends out the burner (./burner)
-- with the damage plus the bonus and lasts until it is done.

local burner = require("./burner")
local dimming_chips = require("../../lib/dimming")
local stand_in = require("../../lib/dimming/stand_in")

export type State = {
    -- The Atk+ / cross bonus the telop shows (+0x32), which the flames'
    -- damage adds.
    bonus: number,
    -- The burner is out (the original's second parameter).
    busy: boolean,
}

-- `sub_80E7070`: the burner on the first tick; the effect ends once it is
-- done (at once if it couldn't be spawned).
local function effect(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        stand_in.spawn(burner.kind, me, me.related1 :: Object, dimming_chips.damage_plus(me, s.bonus))
        me.phase_init = 4
    end
    if not s.busy then
        dimming_chips.done(me)
    end
end

local kind = define.kind {
    id = "bn6:burnsqr/controller",
    pool = "effect",
    state = { bonus = "u16", busy = "bool" },
    -- `sub_80E702C`.
    update = dimming_chips.phases { name = "BurnSqr", effect = effect },
}

-- The chips' `dimming` hook (`sub_80E70A6`): the controller, of the user's
-- side (its facing not copied).
local function hook(user: Object, spec: DimmingChipSpec): Object?
    local o = dimming_chips.spawn(kind, user, spec, true)
    if o then
        (o.state :: State).bonus = spec.bonus
    end
    return o
end

return { hook = hook }
