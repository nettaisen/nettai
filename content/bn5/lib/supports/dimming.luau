--!strict
-- The supports' controller as a chip's dimming (the dimming handler's subtype 50, 0x080E906A: the spawner the
-- NaviCust supports' triggers call, which the handler's table lists too): the controller (./controller) on the user's
-- panel, the user its host, the support the record's first parameter (0 Rush, 1 Beat, 2 Tango), and the telop's chip
-- the chip itself (+0x30). BN5's records 0x17F, 0x180 and 0x181 name it (PrpCapsl, PnkCapsl and HealBall), and
-- nothing starts them: the first two are MeddySoul's capsules, which never reach a hand. Its position is the
-- handler's registers: the user's row, the element and the hook's own address (the handler's `bx r3`, Thumb).

local controller = require("./controller")
local dimming_chips = require("@exelib/dimming")

local HOOK_ADDRESS = 0x080E906B

-- The controller's state fields this sets (@exelib/supports/controller's).
type Support = "rush" | "beat" | "tango"
type State = { support: Support, telop_chip: Chip? }

local supports = {}

-- A chip's `dimming` hook: `support` comes for the user.
function supports.hook(support: Support): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(controller.kind, user, spec)
        if o then
            o.pos = Vec3.new(user.panel_y, spec.element, HOOK_ADDRESS)
            local s = o.state :: State
            s.support = support
            s.telop_chip = spec.chip
        end
        return o
    end
end

return supports
