--!strict
-- Magnum's dimming controller (the original's effect object #0x5B,
-- `sub_80E71C4`; docs/engine/dimming-chip-effects.md §11): the usual
-- dimming phases (@exelib/dimming), with an effect that sends out the gunner
-- (./gunner) with the damage plus the bonus and lasts until it is done.

local dimming_chips = require("@exelib/dimming")
local gunner = require("./gunner")
local stand_in = require("@exelib/dimming/stand_in")

export type State = {
    -- The Atk+ / cross bonus the telop shows (+0x32), which the shot's
    -- damage adds.
    bonus: number,
    -- The gunner is out (the original's second parameter).
    busy: boolean,
}

-- `sub_80E7208`: the gunner on the first tick; the effect ends once it is
-- done (at once if it couldn't be spawned).
local function effect(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        stand_in.spawn(gunner.kind, me, me.related1 :: Object, dimming_chips.damage_plus(me, s.bonus))
        me.phase_init = 4
    end
    if not s.busy then
        dimming_chips.done(me)
    end
end

local kind = define.kind {
    id = "magnum/controller",
    pool = "effect",
    state = { bonus = "u16", busy = "bool" },
    -- `sub_80E71C4`.
    update = dimming_chips.phases { name = "Magnum", effect = effect },
}

-- The chip's `dimming` hook (`sub_80E723E`): the controller, of the user's
-- side (its facing not copied).
local function hook(user: Object, spec: DimmingChipSpec): Object?
    local o = dimming_chips.spawn(kind, user, spec, true)
    if o then
        (o.state :: State).bonus = spec.bonus
    end
    return o
end

return { hook = hook }
