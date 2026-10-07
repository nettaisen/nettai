--!strict
-- Phoenix's and DethPhnx's dimming controllers (EXE5's effect objects #0x0D, 0x080E1590, and #0x15, 0x080E1E9C):
-- @exelib/navi_chips/controller's, with EXE5's course (0x080E1F08): 31 ticks; the chip's phoenix, which it waits on;
-- 31 ticks; 31 more. The user stays on the field throughout (EXE6's navi chip's controller warps it out and back), and
-- nothing is kept as the last navi chip. Each chip makes its own (`controllers.make`: ./controller,
-- ../dethphnx/controller).

local common = require("@exelib/navi_chips/controller")

local controllers = {}

export type State = NaviChipControllerState

export type Spec = {
    -- The kind's id, and its name for the errors past its tables.
    id: string,
    name: string,
    -- The chip's phoenix for `user`, on the controller's panel, of the user's side and facing, with the damage word
    -- (bonus included), reporting to the controller (its `done`): EXE5's 0x080BFCC4 and 0x080C00B6.
    spawn: (controller: Object, user: Object, damage: number) -> Object?,
}

local WAIT = 0x1E
local STEPS: { NaviChipStep } = { { wait = WAIT }, "navi", { wait = WAIT }, { wait = WAIT } }

-- The phoenix is done (it writes 0 through the pointer its spawn was given).
controllers.done = common.done

function controllers.make(spec: Spec): NaviChipController
    return common.make { id = spec.id, name = spec.name, spawn = spec.spawn, steps = STEPS }
end

return controllers
