--!strict
-- EXE5's Geddon controller (effect object #0x1D, 0x080E2718): EXE6's (@exelib/geddon/controller) without holding the
-- user's panel (0x080E275C), setting off the shared quake (@exelib/geddon/quake, EXE5's effect #0x1E, the same code:
-- 0x080E27F0; its rows, by the chip's first parameter, crack, break or poison every solid panel) with EXE5's rising
-- bubble and sounds (its puff the sheet 14-02, EXE6's small puff).

local common = require("@exelib/geddon/controller")
local quake = require("@exelib/geddon/quake")
local rising_bubble = require("../../objects/rising_bubble")

local quakes = quake.make {
    id = "geddon/quake",
    puff = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0 },
    bubble = rising_bubble.kind,
    sounds = { puff = asset.sound("panel-crack"), bubble = asset.sound("panel-poison"), poison = asset.sound("panel-poison") },
}

return common.make { id = "geddon/controller", name = "Geddon", quake = quakes.spawn, hold_panel = false }
