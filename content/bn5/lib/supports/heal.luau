--!strict
-- Tango's heal (BN5's attack object #0x9F, 0x080DA6AC; BN6's #0xC7): the shared heal (@common/supports/heal, BN6's
-- code) in BN5's look: it arcs onto Tango's navi's panel, heals it 300 HP and gives it BN5's 100-HP barrier (type 5,
-- lib/barriers) with BN5's barrier visual. See docs/engine/chips.md §2.10.

local barriers = require("../barriers")
local common = require("@common/supports/heal")
local trajectory = require("../trajectory")

return common.make {
    id = "bn5:tango/heal",
    sprite = asset.sprite("bn5:tango-heal"),
    sound = asset.sound("bn5:heal"),
    -- The sparkle as it lands (BN5's effect 6, BN6's row: a recovery's).
    sparkle = define.effect { sprite = asset.sprite("bn5:heal"), anim = 0, palette = 0 },
    trajectory = trajectory,
    -- 0x080174DA(5), then the barrier's visual (0x080DA77A).
    raise_barrier = function(navi: Object)
        barriers.raise(navi, barriers.barrier_100)
    end,
}
