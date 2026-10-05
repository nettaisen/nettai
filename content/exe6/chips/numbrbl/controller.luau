--!strict
-- NumbrBl's dimming controller (the original's effect object #0x69,
-- `sub_80E7F38`; docs/engine/dimming-chip-effects.md §15): the one EXE5
-- shares (@exelib/numbrbl/controller), with EXE6's effect, which sends
-- NumberMan (./numberman) out in the user's place and waits until he is
-- done.
--
-- NumbrBl's `dimming` hook is `controller.hook(balls)`.

local common = require("@exelib/numbrbl/controller")
local dimming_chips = require("@exelib/dimming")
local numberman = require("./numberman")

local controller = {}

-- `sub_80E7F80`: NumberMan; once he is done, undim.
local function effect(me: Object)
    local s = me.state :: any
    if me.phase_init == 0 then
        numberman.spawn(me, me.related1 :: Object, dimming_chips.damage_plus(me, s.bonus), s.balls)
        me.phase_init = 4
    end
    if not s.busy then
        me:set_action(common.UNDIM)
    end
end

controller.kind = common.make("numbrbl/controller", effect)

-- `sub_80E7FBA`: NumbrBl's `dimming` hook, for `balls` balls.
function controller.hook(balls: number): (user: Object, spec: DimmingChipSpec) -> Object?
    return common.hook(controller.kind, balls)
end

return controller
