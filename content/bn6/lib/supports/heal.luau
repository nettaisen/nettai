--!strict
-- Tango's heal (the original's attack object #0xC7, `sub_80DE000`): the shared heal (@exelib/supports/heal, BN5's
-- too) in BN6's look: it arcs onto Tango's navi's panel, heals it 300 HP and gives it Barr100's barrier (lib/barriers)
-- with the barrier's visual. See docs/engine/chips.md §2.10.

local barriers = require("../barriers")
local common = require("@exelib/supports/heal")
local trajectory = require("../trajectory")

return common.make {
    id = "tango/heal",
    sprite = asset.sprite("tango-heal"),
    sound = asset.sound("heal"),
    -- The sparkle as it lands (the original's effect 6: a recovery's).
    sparkle = define.effect { sprite = asset.sprite("heal"), anim = 0, palette = 0 },
    trajectory = trajectory,
    -- `sub_801A7CC(5)`, then the barrier's visual.
    raise_barrier = function(navi: Object)
        barriers.raise(navi, barriers.barrier_100)
    end,
}
