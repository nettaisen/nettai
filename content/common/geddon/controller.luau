--!strict
-- Geddon's dimming controller, as BN5 and BN6 share it (BN6's effect object #0x1D, `sub_80E24E4`, BN5's 0x1D,
-- 0x080E275C): the usual dimming phases (BN6's lib/dimming), with an effect that sets off a quake over the field
-- and waits until it is done. BN6's holds the user's panel while the quake runs (it reserves it, so it counts as
-- occupied); BN5's doesn't (0x080E275C: no `object_reservePanel` or `object_removePanelReserve`).
--
-- Each game makes its own kind (`controller.make`) with its quake's spawn; a chip's `dimming` hook is the made
-- controller's `hook(change)`.

local dimming_chips = require("@bn6/lib/dimming")

local controller = {}

-- What the quake does to the panels (its rows, by the chip's first parameter).
export type Change = "crack" | "break" | "poison"

export type State = {
    change: Change,
    -- The quake runs (the original's second parameter, whose address it holds).
    busy: boolean,
}

export type Spec = {
    -- The kind's id.
    id: string,
    -- For the error past the action table.
    name: string,
    -- Spawns the quake of `change` on `spawner`'s side, which clears `holder`'s `busy` when it is done.
    quake: (spawner: Object, change: Change, holder: Object) -> Object?,
    -- Whether the user's panel is held while the quake runs (BN6's).
    hold_panel: boolean,
}

export type Controller = {
    kind: Kind,
    hook: (change: Change) -> (user: Object, spec: DimmingChipSpec) -> Object?,
}

function controller.make(spec: Spec): Controller
    local hold = spec.hold_panel
    -- BN6's `sub_80E2528`, BN5's 0x080E275C: (the user's panel held and) the quake; once it is done, (the panel
    -- let go and) the effect is over.
    local function effect(me: Object)
        local s = me.state :: State
        local user = me.related1 :: Object
        if me.phase_init == 0 then
            if hold then
                me:reserve_panel(user.panel_x, user.panel_y)
            end
            -- Set before the spawn: a quake that fails to spawn leaves the dimming waiting for good, as in the
            -- original.
            s.busy = true
            spec.quake(me, s.change, me)
            me.phase_init = 4
        end
        if not s.busy then
            if hold then
                me:unreserve_panel(user.panel_x, user.panel_y)
            end
            dimming_chips.done(me)
        end
    end

    local kind = define.kind {
        id = spec.id,
        pool = "effect",
        state = { change = { "crack", "break", "poison" }, busy = "bool" },
        -- BN6's `sub_80E24E4`.
        update = dimming_chips.phases { name = spec.name, effect = effect },
    }

    -- BN6's `sub_80E2566`, BN5's 0x080E2786: a chip's `dimming` hook, for the quake's change. (The spawner copies
    -- the user's side, not its facing.)
    local function hook(change: Change): (user: Object, dspec: DimmingChipSpec) -> Object?
        return function(user: Object, dspec: DimmingChipSpec): Object?
            local o = dimming_chips.spawn(kind, user, dspec, true)
            if o then
                (o.state :: State).change = change
            end
            return o
        end
    end

    return { kind = kind, hook = hook }
end

return controller
