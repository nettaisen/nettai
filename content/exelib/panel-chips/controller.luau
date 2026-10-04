--!strict
-- The panel chips' dimming controller, as BN5 and BN6 share it (BN6's
-- effect object #0x20, `sub_80E2AE8`; BN5's #0x20;
-- docs/engine/dimming-chips.md §4.1): PnlRetrn, HolyPanl, Snctuary,
-- ComingRd and GoingRd. Each game makes its own (`panel_chips.make`: its
-- kind and its panel changer's). The usual dimming phases
-- (@exelib/dimming), with an effect (`sub_80E2B2C`) that hands the chip's
-- panel change to a panel changer (@exelib/panel-changer) on the user's
-- panel, and waits until it is done.
--
-- A chip's `dimming` hook is the made controller's `hook(change)`, with its
-- change.

local dimming_chips = require("../dimming")
local panel_changer = require("@exelib/panel-changer")

local panel_chips = {}

export type State = {
    change: PanelChange,
    -- The changer runs (the original's Param2, whose address it holds).
    busy: boolean,
}

-- A game's controller: its kind's key, and its panel changer's kind.
export type Spec = { id: string, changer: Kind }

export type PanelChips = {
    kind: Kind,
    -- `sub_80E2B5A`: a chip's `dimming` hook, for its change.
    hook: (change: PanelChange) -> (user: Object, spec: DimmingChipSpec) -> Object?,
}

-- A game's controller.
function panel_chips.make(spec: Spec): PanelChips
    -- `sub_80E2B2C`: the changer, then wait.
    local function effect(me: Object)
        local s = me.state :: State
        if me.phase_init == 0 then
            s.busy = true
            panel_changer.spawn(spec.changer, me, me.panel_x, me.panel_y, s.change, me)
            me.phase_init = 4
        end
        if not s.busy then
            dimming_chips.done(me)
        end
    end

    local kind = define.kind {
        id = spec.id,
        pool = "effect",
        state = { change = "record:panel-change", busy = "bool" },
        update = dimming_chips.phases { name = "the panel chips", effect = effect },
    }

    local function hook(change: PanelChange): (user: Object, spec: DimmingChipSpec) -> Object?
        return function(user: Object, chip: DimmingChipSpec): Object?
            local o = dimming_chips.spawn(kind, user, chip)
            if o then
                (o.state :: State).change = change
            end
            return o
        end
    end

    return { kind = kind, hook = hook }
end

-- The conditions most changes use: a side's own area (side 0's: not
-- missing, not side 1's; side 1's: side 1's, not missing), and any panel
-- that isn't missing.
panel_chips.OWN_AREA = { [0] = { require = 0x0, forbid = 0x8020 }, [1] = { require = 0x20, forbid = 0x8000 } }
panel_chips.OTHER_AREA = { [0] = { require = 0x20, forbid = 0x8000 }, [1] = { require = 0x0, forbid = 0x8020 } }
panel_chips.ANY = { [0] = { require = 0x0, forbid = 0x8000 }, [1] = { require = 0x0, forbid = 0x8000 } }

return panel_chips
