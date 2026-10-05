--!strict
-- BlzrdBal's dimming controller (the original's effect object #0x58,
-- `sub_80E6F88`; docs/engine/dimming-chip-effects.md §10): the usual
-- dimming phases (@exelib/dimming), with an effect that sends out the thrower
-- (./thrower) with the damage plus the bonus and lasts 121 ticks, whether
-- or not it is done.

local dimming_chips = require("@exelib/dimming")
local stand_in = require("@exelib/dimming/stand_in")
local thrower = require("./thrower")

export type State = {
    -- The Atk+ / cross bonus the telop shows (+0x32), which the ball's
    -- damage adds.
    bonus: number,
    -- The thrower is out (the original's second parameter, which nothing
    -- here reads).
    busy: boolean,
}

local DURATION = 0x78

-- `sub_80E6FCC`: the thrower on the first tick; 121 ticks.
local function effect(me: Object)
    if me.phase_init == 0 then
        me.timer = DURATION
        local s = me.state :: State
        stand_in.spawn(thrower.kind, me, me.related1 :: Object, dimming_chips.damage_plus(me, s.bonus))
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        dimming_chips.done(me)
    end
end

local kind = define.kind {
    id = "blzrdbal/controller",
    pool = "effect",
    state = { bonus = "u16", busy = "bool" },
    -- `sub_80E6F88`.
    update = dimming_chips.phases { name = "BlzrdBal", effect = effect },
}

-- The chip's `dimming` hook (`sub_80E7008`): the controller, of the user's
-- side (its facing not copied).
local function hook(user: Object, spec: DimmingChipSpec): Object?
    local o = dimming_chips.spawn(kind, user, spec, true)
    if o then
        (o.state :: State).bonus = spec.bonus
    end
    return o
end

return { hook = hook }
