--!strict
-- BN5's GrabBnsh's and GrabRvng's dimming controller (effect object #0x22) and hand (attack object #0x46): BN6's
-- code (@common/grabbnsh), BN5's kinds; the hand's look BN6's, its hit BN6's curse row (with the 0x80 self bit)
-- hitting navis, the flash on each panel taken
-- back (BN5's effect 0x29, BN6's 0x3B: the same) and its sound BN6's.

local bn6_collision = require("@bn6/rules/collision")
local collision = require("../../rules/collision")
local common = require("@common/grabbnsh/controller")
local hand = require("@common/grabbnsh/hand")

local hand_kind = hand.make_kind {
    id = "bn5:grabbnsh/hand",
    sprite = asset.sprite("bn6:grab-shot"),
    fall_sound = asset.sound("bn6:grab-shot-2"),
    land_sound = asset.sound("bn6:grab-shot"),
    target = collision.hits_navis,
    self_type = bn6_collision.curse,
}

return common.make {
    id = "bn5:grabbnsh/controller",
    hand = hand_kind,
    returned = define.effect { sprite = asset.sprite("bn6:grab-shot"), anim = 1, palette = 0 },
    return_sound = asset.sound("bn6:err-select-91"),
}
