--!strict
-- GrabBnsh's and GrabRvng's hand (the original's attack object #0x46,
-- `sub_80CCC48`): the shared hand (@common/grabbnsh/hand), BN6's kind and
-- look: its hit an attack that pierces invisibility (row 0x19), hitting
-- navis.

local collision = require("../../rules/collision")
local common = require("@common/grabbnsh/hand")

local hand = {}

hand.kind = common.make_kind {
    id = "bn6:grabbnsh/hand",
    sprite = asset.sprite("bn6:grab-shot"),
    fall_sound = asset.sound("bn6:grab-shot-2"),
    land_sound = asset.sound("bn6:grab-shot"),
    target = collision.hits_navis,
    self_type = collision.curse,
}

return hand
