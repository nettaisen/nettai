--!strict
-- Beat (BN5's actor object #0x4C, 0x080C24C8): the shared Beat (@common/supports/beat, BN6's code) in BN5's look, the
-- NaviCust support that snatches a Mega or Giga chip (0x0800E418). See docs/engine/chips.md §2.10.

local common = require("@common/supports/beat")

return common.make {
    id = "bn5:beat",
    sprite = asset.sprite("bn5:beat"),
    swoop_sound = asset.sound("bn5:set-down"),
    snatch_sound = asset.sound("bn5:snatch"),
}
