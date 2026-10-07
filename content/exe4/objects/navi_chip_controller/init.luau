--!strict
-- The navi chip controller six of EXE4's navis share (effect object #0x42, 0x080E6A18): the dimming chips' action 0x0C,
-- its variants 0x20 (SerchMan), 0x23 (ThunMan), 0x2F (ProtoMan), 0x32 (DeltaRay), 0x3C (MetalMan) and 0x40 (JunkMan),
-- whose spawner is one (0x080E6B22). lib/navi_chips' controller with the course `SHARED` (the user warps out, the
-- navi, the user warps back in); its navi by the chip's third parameter (0x080E6AE0's entry: 0 SerchMan, 4 ThunMan,
-- 8 ProtoMan, 0xC DeltaRay, 0x10 MetalMan, 0x14 JunkMan), its level by the first. A chip's `dimming` hook is
-- `controller.hook_for(level, which)`.

local deltaray = require("../chips/deltaray/navi")
local metalman = require("../chips/metalman/navi")
local navi_chips = require("../lib/navi_chips")
local protoman = require("../chips/protoman/navi")
local serchman = require("../chips/serchman/navi")
local thunman = require("../chips/thunman/navi")

-- The navis by the chip's third parameter (0x080E6AE0).
local NAVIS: { [number]: (controller: Object, user: Object, damage: number) -> Object? } = {
    [0] = serchman.spawn,
    [4] = thunman.spawn,
    [8] = protoman.spawn,
    [0xC] = deltaray.spawn,
    [0x10] = metalman.spawn,
}

-- 0x080E6AA6: the navi the chip's third parameter names.
local function spawn(controller: Object, user: Object, damage: number): Object?
    local which = (controller.state :: NaviChipControllerState).which
    local navi = NAVIS[which]
    if not navi then
        error(string.format("the shared navi chip controller's navi %#x reads past 0x080E6AE0", which))
    end
    return navi(controller, user, damage)
end

return navi_chips.controller {
    id = "navi-chip-controller",
    name = "The shared navi chip",
    spawn = spawn,
    steps = navi_chips.SHARED,
}
