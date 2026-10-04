--!strict
-- MoonBld's blade (attack object #0x85, `sub_80D5C48`): the shared blade
-- (@exelib/moonbld/blade) with BN6's sprite and collision types. It rides
-- on its owner and sweeps the eight panels around it, one a tick for 8
-- ticks, starting ahead and going round, then goes after 10 ticks.

local collision = require("../../rules/collision")
local common = require("@exelib/moonbld/blade")

local blade = {}

blade.kind = common.make_kind("bn6:moonbld/blade", {
    sprite = asset.sprite("bn6:moon-blade"),
    is = collision.slash,
    hits = collision.hits_navis,
})

-- `sub_80D5D34`: a blade on `owner` (`plain`: a plain blade's hits), its
-- panel and position the spawner's registers (`routine`: the address of the
-- phase routine spawning it; see @exelib/moonbld/blade).
function blade.spawn(owner: Object, panel_x: number, routine: number, damage: number, plain: boolean): Object?
    return common.spawn(blade.kind, owner, panel_x, routine, damage, plain)
end

return blade
