--!strict
-- BN5's Geddon controller (effect object #0x1D, 0x080E2718): BN6's (@exelib/geddon/controller) without holding the
-- user's panel (0x080E275C), setting off the shared quake (@exelib/geddon/quake, BN5's effect #0x1E, the same code:
-- 0x080E27F0; its rows, by the chip's first parameter, crack, break or poison every solid panel) with BN5's rising
-- bubble and sounds (its puff the sheet 14-02, BN6's small puff).

local common = require("@exelib/geddon/controller")
local quake = require("@exelib/geddon/quake")
local rising_bubble = require("../../objects/rising-bubble/rising_bubble")

local quakes = quake.make {
    id = "bn5:geddon/quake",
    puff = define.effect { sprite = asset.sprite("bn5:small-puff"), anim = 1, palette = 0 },
    bubble = rising_bubble.kind,
    sounds = { puff = asset.sound("bn5:panel-crack"), bubble = asset.sound("bn5:panel-poison") },
}

return common.make { id = "bn5:geddon/controller", name = "Geddon", quake = quakes.spawn, hold_panel = false }
