--!strict
-- LifeSync's marker (the original's effect object #0x60, `sub_80E76F8`;
-- docs/engine/dimming-chip-effects.md §14): the shared marker
-- (@exelib/lifesync/marker) in EXE6's hit sparks. A cursor over the panel
-- two ahead of the user, then over the enemy there, for 81 ticks. Only
-- drawn; LifeSync's controller places it outside link battles only.

local common = require("@exelib/lifesync/marker")

local marker = {}

marker.kind = common.make_kind("lifesync/marker", asset.sprite("hit-sparks"))

-- `sub_80E7774`: a marker at `pos` for `ticks` ticks.
function marker.spawn(pos: Vec3, ticks: number): Object?
    return common.spawn(marker.kind, pos, ticks)
end

return marker
