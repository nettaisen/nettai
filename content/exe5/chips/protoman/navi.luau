--!strict
-- EXE5's ProtoMan (actor object #0x3C, EXE6's code: @exelib/protoman/navi), EXE5's kind: his attachment EXE5's, his
-- sounds EXE6's, his slash an EXE6 slash (EXE6's row, with its 0x80 self bit) hitting navis.

local attachment = require("../../objects/attachment")
local collision = require("../../rules/collision")
local common = require("@exelib/protoman/navi")

return common.make {
    id = "protoman/navi",
    appear_sound = asset.sound("appear"),
    swing_sound = asset.sound("sword-swing"),
    attachment = attachment.kind,
    target = collision.hits_navis,
    self_type = collision.slash,
}
