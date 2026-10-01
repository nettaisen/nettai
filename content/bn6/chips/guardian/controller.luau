--!strict
-- The Guardian chip's dimming controller (the original's effect object
-- #0x52, `sub_80E6758`; docs/engine/dimming-chip-effects.md §5): the usual
-- dimming phases, with an effect that places a Guardian statue (./statue)
-- on the panel in front of the user if it is free solid ground, striking
-- with the damage plus the bonus, and waits 30 ticks.

local dimming_chips = require("../../lib/dimming")
local statue = require("./statue")

export type State = {
    -- The Atk+ / cross bonus the telop shows (+0x32), which the statue's
    -- damage adds.
    bonus: number,
}

local WAIT = 0x1E
-- Where the statue goes: a solid panel with nothing on it.
local SOLID, OCCUPIED = 0x10, 0x0F88_0080

-- `sub_80E679C`: the statue, then 30 ticks.
local function effect(me: Object)
    if me.phase_init == 0 then
        local x, y = me.panel_x + me.facing, me.panel_y
        if field.check(x, y, SOLID, OCCUPIED) then
            local bonus = (me.state :: State).bonus
            statue.spawn(me, x, y, dimming_chips.damage_plus(me, bonus), Vec3.new(y, me.element, bonus))
        end
        me.timer = WAIT
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t == 0 then
        dimming_chips.done(me)
    end
end

local kind = define.kind {
    id = "guardian/controller",
    pool = "effect",
    state = { bonus = "u16" },
    -- `sub_80E6758`.
    update = dimming_chips.phases { name = "Guardian's controller", effect = effect },
}

-- The chip's `dimming` hook (`sub_80E67E6`).
local function hook(user: Object, spec: DimmingChipSpec): Object?
    local o = dimming_chips.spawn(kind, user, spec)
    if o then
        (o.state :: State).bonus = spec.bonus
    end
    return o
end

return { hook = hook }
