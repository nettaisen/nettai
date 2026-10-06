--!strict
-- Tango's heal (EXE5's attack object #0x9F, 0x080DA6AC; EXE6's #0xC7): the shared heal (@exelib/supports/heal, EXE6's
-- code) in EXE5's look: it arcs onto Tango's navi's panel, heals it 300 HP and gives it EXE5's 100-HP barrier (type 5,
-- lib/barriers) with EXE5's barrier visual. See docs/engine/chips.md §2.10.

local barriers = require("../barriers")
local common = require("@exelib/supports/heal")
local trajectory = require("../trajectory")

return common.make {
    id = "tango/heal",
    sprite = asset.sprite("tango-heal"),
    sound = asset.sound("heal"),
    -- The sparkle as it lands (EXE5's effect 6, EXE6's row: a recovery's).
    sparkle = new.effect { sprite = asset.sprite("heal"), anim = 0, palette = 0 },
    trajectory = trajectory,
    -- 0x080174DA(5), then the barrier's visual (0x080DA77A).
    raise_barrier = function(navi: Object)
        barriers.raise(navi, barriers.barrier_100)
    end,
}
