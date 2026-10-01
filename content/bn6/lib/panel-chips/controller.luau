--!strict
-- The panel chips' dimming controller (the original's effect object
-- #0x20, `sub_80E2AE8`; docs/engine/dimming-chips.md §4.1): PnlRetrn,
-- HolyPanl, Snctuary, ComingRd and GoingRd. The usual dimming phases
-- (lib/dimming), with an effect (`sub_80E2B2C`) that hands the chip's
-- panel change to a panel changer (objects/panel-changer) on the user's
-- panel, and waits until it is done.
--
-- A chip's `dimming` hook is `panel_chips.hook(change)`, with its change.

local controller = require("../dimming/controller")
local panel_changer = require("../../objects/panel-changer/panel_changer")

local panel_chips = {}

export type State = {
    change: PanelChange,
    -- The changer runs (the original's Param2, whose address it holds).
    busy: boolean,
}

-- `sub_80E2B2C`: the changer, then wait.
local function effect(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        s.busy = true
        panel_changer.spawn(me, me.panel_x, me.panel_y, s.change, me)
        me.phase_init = 4
    end
    if not s.busy then
        controller.undim(me)
    end
end

panel_chips.kind = define.kind {
    id = "panel-chips/controller",
    pool = "effect",
    state = { change = "record:panel-change", busy = "bool" },
    update = controller.update("the panel chips' controller", effect),
}

-- `sub_80E2B5A`: a chip's `dimming` hook, for its change.
function panel_chips.hook(change: PanelChange): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = controller.spawn(panel_chips.kind, user, spec)
        if o then
            (o.state :: State).change = change
        end
        return o
    end
end

-- The conditions most changes use: a side's own area (side 0's: not
-- missing, not side 1's; side 1's: side 1's, not missing), and any panel
-- that isn't missing.
panel_chips.OWN_AREA = { [0] = { require = 0x0, forbid = 0x8020 }, [1] = { require = 0x20, forbid = 0x8000 } }
panel_chips.OTHER_AREA = { [0] = { require = 0x20, forbid = 0x8000 }, [1] = { require = 0x0, forbid = 0x8020 } }
panel_chips.ANY = { [0] = { require = 0x0, forbid = 0x8000 }, [1] = { require = 0x0, forbid = 0x8000 } }

return panel_chips
