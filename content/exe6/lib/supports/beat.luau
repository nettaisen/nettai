--!strict
-- Beat (the original's actor object #0x4C, `sub_80C34E0`): the shared Beat (@exelib/supports/beat, EXE5's too) in
-- EXE6's look, the NaviCust support that snatches a Mega or Giga chip (`sub_80106C0`). See docs/engine/chips.md §2.10.

local common = require("@exelib/supports/beat")

return common.make {
    id = "beat",
    sprite = asset.sprite("beat"),
    swoop_sound = asset.sound("set-down"),
    snatch_sound = asset.sound("snatch"),
}
