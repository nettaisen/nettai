--!strict
-- EXE6's trap chips' dimming controller (the original's effect object #0x2A,
-- `sub_80E34C0`): the shared controller (@exelib/traps/controller), EXE6's
-- kind. A chip's `dimming` hook is `traps.hook(trap?)` (`sub_80E353E`,
-- `off_802CCB4[20]`): the trap the chip sets, if any.

local common = require("@exelib/traps/controller")

local traps = {}

traps.kind = common.make_kind("trap-chip")

-- A trap's object, as a record a controller can keep.
traps.trap = common.trap

-- A chip's `dimming` hook: EXE6's controller, setting `trap`'s object.
function traps.hook(trap: Trap?): (user: Object, spec: DimmingChipSpec) -> Object?
    return common.hook(traps.kind, trap)
end

return traps
