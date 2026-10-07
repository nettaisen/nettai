--!strict
-- EXE4's trap chips' dimming controller (effect object 0x2A, 0x080E50D0: EXE6's code, @exelib/traps/controller),
-- EXE4's kind: the Anti chips' (the dimming chips' action 0x0C, its variant 0x13, 0x080E514E). The chip's first
-- parameter picks what it sets (0x080E5184): AntiFire to AntiWood's trap (./anti_trap, 0x080E51A4), the others
-- nothing (0x080E51B6) but the side's defensive-chip record.

local common = require("@exelib/traps/controller")

local traps = {}

traps.kind = common.make_kind("trap-chip")

-- A trap's object, as a record a controller can keep.
traps.trap = common.trap

-- A chip's `dimming` hook: EXE4's controller, setting `trap`'s object.
function traps.hook(trap: Trap?): (user: Object, spec: DimmingChipSpec) -> Object?
    return common.hook(traps.kind, trap)
end

return traps
