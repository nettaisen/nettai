--!strict
-- A falling meteor's panel marker (the original's effect object #0x35,
-- `sub_80E4344`): the shared marker (@common/meteors/marker), BN6's kind.

local common = require("@common/meteors/marker")

local marker = {}

-- A meteor's: 45 ticks, lit 5 of every 10.
marker.meteor = common.meteor

marker.kind = common.make_kind("bn6:meteors/marker")

-- `sub_80E43F6`: a marker of `look` highlighting `region` on (x, y), of
-- `owner`'s side.
function marker.spawn(owner: Object, x: number, y: number, look: MeteorMarkerLook, region: Region?): Object?
    return common.spawn(marker.kind, owner, x, y, look, region)
end

return marker
