--!strict
-- BN5's ProtoMan (actor object #0x3C, BN6's code: @exelib/protoman/navi), BN5's kind: his attachment BN5's, his
-- sounds BN6's, his slash a BN6 slash (BN6's row, with its 0x80 self bit) hitting navis.

local attachment = require("../../objects/attachment/attachment")
local collision = require("../../rules/collision")
local common = require("@exelib/protoman/navi")

return common.make {
    id = "bn5:protoman/navi",
    appear_sound = asset.sound("bn5:appear"),
    swing_sound = asset.sound("bn5:sword-swing"),
    attachment = attachment.kind,
    target = collision.hits_navis,
    self_type = collision.slash,
}
