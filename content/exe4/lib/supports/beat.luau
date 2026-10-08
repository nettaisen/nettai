--!strict
-- Beat (EXE4's actor object #0x4C, 0x080CA76C): the shared Beat (@exelib/supports/beat, EXE6's code) in EXE4's look, the
-- NaviCust support that snatches a Mega or Giga chip that isn't dark (0x0800C838; the rules' `chip_use.beat_spares_dark`).
-- See docs/engine/chips.md §2.10.

local common = require("@exelib/supports/beat")

return common.make {
    id = "beat",
    sprite = asset.sprite("beat"),
    swoop_sound = asset.sound("set-down"),
    snatch_sound = asset.sound("snatch"),
}
