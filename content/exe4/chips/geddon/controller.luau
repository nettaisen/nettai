--!strict
-- EXE4's Geddon controller (effect object #0x1D, 0x080E400C; its effect 0x080E4050): EXE6's (@exelib/geddon/controller)
-- without holding the user's panel, as EXE5's, setting off the shared quake (@exelib/geddon/quake, EXE4's effect #0x1E,
-- 0x080E40E4: EXE6's code; its rows, 0x080E409C by the chip's first parameter, crack, break or poison every solid
-- panel: each chip's change, ./init) with EXE4's rising bubble and sounds (its puff the effect row 2, 14-02's
-- animation 1, with 0x95; a bubble with 0x124, 0x080E418A), which the controller gives as `quake`. Its spawn
-- (0x080E407A) takes the user's side, not its facing.

local common = require("@exelib/geddon/controller")
local quake = require("@exelib/geddon/quake")
local rising_bubble = require("../../objects/rising_bubble")

local quakes = quake.make {
    id = "geddon/quake",
    puff = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0 },
    bubble = rising_bubble.kind,
    sounds = { puff = asset.sound("panel-crack"), bubble = asset.sound("panel-poison"), poison = asset.sound("panel-poison") },
}

local made = common.make { id = "geddon/controller", name = "Geddon", quake = quakes.spawn, hold_panel = false }

return { kind = made.kind, hook = made.hook, quake = quakes }
