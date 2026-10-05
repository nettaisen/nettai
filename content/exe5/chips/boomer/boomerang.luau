--!strict
-- EXE5's boomerang (attack object #0x32, EXE6's code: @exelib/boomer/boomerang), EXE5's kind: EXE5's sprite, its sound
-- EXE6's (each variant gives its own sprite, collision type and spark).

local collision = require("../../rules/collision")
local common = require("@exelib/boomer/boomerang")
local sparks = require("../../lib/sparks")

return common.make_kind {
    id = "boomer/boomerang",
    sprite = asset.sprite("boomerang"),
    tomahawk_sprite = asset.sprite("boomerang"),
    sound = asset.sound("boomerang"),
    is = collision.attack,
    hits = collision.hits_navis,
    spark = sparks.wood,
}
