--!strict
-- NumbrBl's dimming controller (EXE5's effect object #0x69, 0x080E82DC): @exelib/numbrbl/controller with EXE5's effect
-- (0x080E8324, `controller.warping`): the user warps out (the engine's `navi_chip.warp`, actor #0x2D); NumberMan
-- himself (./numberman) on the user's panel, until he is done; 30 ticks; the user warps back in; then the undim.

local common = require("@exelib/numbrbl/controller")
local numberman = require("./numberman")

local controller = {}

controller.kind = common.make(
    "numbrbl/controller",
    common.warping(function(me: Object, out: boolean)
        navi_chip.warp(me.related1 :: Object, out)
    end, function(me: Object, damage: number, balls: number)
        numberman.spawn(me, me.panel_x, me.panel_y, damage, balls)
    end)
)

-- 0x080E83E6: NumbrBl's `dimming` hook, NumberMan setting `balls` balls down.
function controller.hook(balls: number): (user: Object, spec: DimmingChipSpec) -> Object?
    return common.hook(controller.kind, balls)
end

return controller
