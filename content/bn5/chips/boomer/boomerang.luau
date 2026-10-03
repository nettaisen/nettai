--!strict
-- BN5's boomerang (attack object #0x32, BN6's code: @common/boomer/boomerang), BN5's kind: BN5's sprite, its sound
-- BN6's (each variant gives its own sprite, collision type and spark).

local collision = require("../../rules/collision")
local common = require("@common/boomer/boomerang")
local sparks = require("../../lib/sparks")

return common.make_kind {
    id = "bn5:boomer/boomerang",
    sprite = asset.sprite("bn5:boomerang"),
    tomahawk_sprite = asset.sprite("bn5:boomerang"),
    sound = asset.sound("bn6:boomerang"),
    is = collision.attack,
    hits = collision.hits_navis,
    spark = sparks.wood,
}
