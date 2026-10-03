--!strict
-- Snake's dimming controller (the original's effect object #0x45,
-- `sub_80E5944`; docs/engine/dimming-chip-effects.md §13): the usual
-- dimming phases (lib/dimming), with an effect that sets a nest (./nest)
-- on the user's panel, waits until its snakes are done, then 41 ticks.

local dimming_chips = require("@common/dimming")
local nest = require("./nest")

local controller = {}

export type State = {
    -- The Atk+ / cross bonus (object +0x32): it adds to the snakes'
    -- damage word.
    bonus: number,
    -- The nest runs (the original's second parameter, whose address it
    -- holds).
    busy: boolean,
}

local AFTER = 0x28

-- `sub_80E5988`: the nest; then, once it is done, 41 ticks.
local function effect(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        nest.spawn(me, me.panel_x, me.panel_y, s.bonus, dimming_chips.damage_plus(me, s.bonus), me)
        -- Set after the spawn, by the controller itself: a nest that fails
        -- to spawn leaves the dimming waiting for good, as in the
        -- original.
        s.busy = true
        me.timer = AFTER
        me.phase_init = 4
    end
    if s.busy then
        return
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        dimming_chips.done(me)
    end
end

controller.kind = define.kind {
    id = "bn6:snake/controller",
    pool = "effect",
    state = { bonus = "u16", busy = "bool" },
    -- `sub_80E5944`.
    update = dimming_chips.phases { name = "Snake", effect = effect },
}

-- `sub_80E59C6`: Snake's `dimming` hook.
function controller.hook(user: Object, spec: DimmingChipSpec): Object?
    local o = dimming_chips.spawn(controller.kind, user, spec)
    if o then
        (o.state :: State).bonus = spec.bonus
    end
    return o
end

return controller
