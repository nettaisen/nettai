--!strict
-- BN5's trap chips' dimming controller (effect object #0x2A, BN6's code: @exelib/traps/controller), BN5's kind.

local common = require("@exelib/traps/controller")

local traps = {}

traps.kind = common.make_kind("trap-chip")

-- A trap's object, as a record a controller can keep.
traps.trap = common.trap

-- A chip's `dimming` hook: BN5's controller, setting `trap`'s object.
function traps.hook(trap: Trap?): (user: Object, spec: DimmingChipSpec) -> Object?
    return common.hook(traps.kind, trap)
end

return traps
