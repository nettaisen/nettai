--!strict
-- Geddon's and the capsules' dimming controller (the original's effect
-- object #0x1D, `sub_80E24E4`; docs/engine/dimming-chip-effects.md §12):
-- the usual dimming phases (lib/dimming), with an effect that holds the
-- user's panel and sets off a quake (./quake) over the field, and waits
-- until it is done.
--
-- A chip's `dimming` hook is `controller.hook(change)`: Geddon's breaks
-- the panels, PrpCapsl's, HealBall's, MagPanl's and the BeastOut records'
-- crack them (the chips' first parameter, 1 or 0, picked the quake's row).

local dimming_chips = require("@common/dimming")
local quake = require("./quake")

local controller = {}

-- What the quake does to the panels (./quake's `Change`).
type Change = "crack" | "break" | "poison"

export type State = {
    change: Change,
    -- The quake runs (the original's second parameter, whose address it
    -- holds).
    busy: boolean,
}

-- `sub_80E2528`: the user's panel held (the controller reserves it, so
-- it counts as occupied) and the quake; once it is done, the panel let go
-- and the effect is over.
local function effect(me: Object)
    local s = me.state :: State
    local user = me.related1 :: Object
    if me.phase_init == 0 then
        me:reserve_panel(user.panel_x, user.panel_y)
        -- Set before the spawn: a quake that fails to spawn leaves the
        -- dimming waiting for good, as in the original.
        s.busy = true
        quake.spawn(me, s.change, me)
        me.phase_init = 4
    end
    if not s.busy then
        me:unreserve_panel(user.panel_x, user.panel_y)
        dimming_chips.done(me)
    end
end

controller.kind = define.kind {
    id = "bn6:geddon/controller",
    pool = "effect",
    state = { change = { "crack", "break", "poison" }, busy = "bool" },
    -- `sub_80E24E4`.
    update = dimming_chips.phases { name = "Geddon", effect = effect },
}

-- `sub_80E2566`: a chip's `dimming` hook, for the quake's change. (The
-- spawner copies the user's side, not its facing.)
function controller.hook(change: Change): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(controller.kind, user, spec, true)
        if o then
            (o.state :: State).change = change
        end
        return o
    end
end

return controller
