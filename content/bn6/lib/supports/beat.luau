--!strict
-- Beat (the original's actor object #0x4C, `sub_80C34E0`): the shared Beat (@common/supports/beat, BN5's too) in
-- BN6's look, the NaviCust support that snatches a Mega or Giga chip (`sub_80106C0`). See docs/engine/chips.md §2.10.

local common = require("@common/supports/beat")

return common.make {
    id = "bn6:beat",
    sprite = asset.sprite("bn6:beat"),
    swoop_sound = asset.sound("bn6:set-down"),
    snatch_sound = asset.sound("bn6:snatch"),
}
