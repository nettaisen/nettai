--!strict
-- EXE5's panel bursts (effect object #0x24, EXE6's code: @exelib/panel-bursts/panel_bursts), EXE5's kind.

local common = require("@exelib/panel-bursts")

local panel_bursts = {}

panel_bursts.kind = common.make_kind("panel-bursts")

-- Bursts of `effect` (with `sound`, if any) over the panels of `region`, one every `interval` ticks at height `z`.
function panel_bursts.spawn(
    spawner: Object,
    region: Region,
    interval: number,
    z: number,
    effect: Effect,
    sound: Sound?,
    x: number?,
    y: number?
): Object?
    return common.spawn(panel_bursts.kind, spawner, region, interval, z, effect, sound, x, y)
end

return panel_bursts
