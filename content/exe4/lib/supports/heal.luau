--!strict
-- Tango's heal (EXE4's attack object #0x9F, 0x080E0CD4; EXE6's #0xC7): the shared heal (@exelib/supports/heal, EXE6's
-- code) in EXE4's look: it arcs onto Tango's navi's panel, heals it 300 HP and gives it EXE4's 100-HP barrier (type 2,
-- lib/barriers) with EXE4's barrier visual, its old one gone first (0x080E0D5C). See docs/engine/chips.md §2.10.

local barriers = require("../barriers")
local common = require("@exelib/supports/heal")
local trajectory = require("../trajectory")

return common.make {
    id = "tango/heal",
    sprite = asset.sprite("tango-heal"),
    sound = asset.sound("heal"),
    -- The sparkle as it lands (effect row 7, 0x080E1698: a recovery's).
    sparkle = new.effect { sprite = asset.sprite("heal"), anim = 0, palette = 0 },
    trajectory = trajectory,
    -- 0x08012DCE(2), then the barrier's visual (0x080E264A on the old one, 0x080E2622).
    raise_barrier = function(navi: Object)
        barriers.raise(navi, barriers.barrier_100)
    end,
}
