--!strict
-- BN5's GrabBnsh's and GrabRvng's dimming controller (effect object #0x22) and hand (attack object #0x46): BN6's
-- code (@common/grabbnsh), BN5's kinds; the hand's look BN6's numbers, its hit BN5's curse row
-- hitting navis, the flash on each panel taken
-- back (BN5's effect 0x29, BN6's 0x3B: the same) and its sound BN6's number.

local collision = require("../../rules/collision")
local common = require("@common/grabbnsh/controller")
local hand = require("@common/grabbnsh/hand")

local hand_kind = hand.make_kind {
    id = "bn5:grabbnsh/hand",
    sprite = asset.sprite("bn5:grab-shot"),
    fall_sound = asset.sound("bn5:grab-shot-2"),
    land_sound = asset.sound("bn5:grab-shot"),
    target = collision.hits_navis,
    self_type = collision.curse,
}

return common.make {
    id = "bn5:grabbnsh/controller",
    hand = hand_kind,
    returned = define.effect { sprite = asset.sprite("bn5:grab-shot"), anim = 1, palette = 0 },
    return_sound = asset.sound("bn5:err-select-91"),
}
