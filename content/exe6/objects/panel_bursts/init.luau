--!strict
-- Panel bursts (the original's effect object #0x24, `sub_80E2F56`): the
-- shared bursts (@exelib/panel_bursts/panel_bursts), EXE6's kind. Its
-- spawner (`sub_80E2FE8`) is `panel_bursts.spawn(spawner, region,
-- interval, z, effect)`.

local common = require("@exelib/panel_bursts")

local panel_bursts = {}

panel_bursts.kind = common.make_kind("panel-bursts")

-- `sub_80E2FE8`: bursts of `effect` (with `sound`, if any) over the panels
-- of `region`, one every `interval` ticks at height `z`.
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
