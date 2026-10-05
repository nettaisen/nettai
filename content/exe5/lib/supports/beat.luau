--!strict
-- Beat (EXE5's actor object #0x4C, 0x080C24C8): the shared Beat (@exelib/supports/beat, EXE6's code) in EXE5's look, the
-- NaviCust support that snatches a Mega or Giga chip (0x0800E418). See docs/engine/chips.md §2.10.

local common = require("@exelib/supports/beat")

return common.make {
    id = "beat",
    sprite = asset.sprite("beat"),
    swoop_sound = asset.sound("set-down"),
    snatch_sound = asset.sound("snatch"),
}
