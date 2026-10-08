--!strict
-- EXE4's VDoll curse (effect object #0x4E, 0x080E77E8): the shared one (@exelib/vdoll/curse, which EXE5's grew from) in
-- EXE4's look: its telop EXE4's Curse (chip 0x174, ../curse); the mark's warning sound (0x08008424: 0x79); its hit
-- (0x080E79B4: a thrown piercing break against navis, the heavy spark, 6); EXE4's sparkles (../../objects/sparkles,
-- staying where their object was) of effect row 0x2B (sheet 14-00 in palette 1).

local collision = require("../../rules/collision")
local common = require("@exelib/vdoll/curse")
local curse_chip = require("../curse").curse
local sparkles = require("../../objects/sparkles")
local sparks = require("../../lib/sparks")

return common.make {
    id = "vdoll/curse",
    chip = curse_chip,
    mark_sound = asset.sound("program-advance-part"),
    spark = sparks.heavy,
    is = collision.thrown_piercing_break,
    hits = collision.hits_navis,
    sparkles = sparkles,
    sparkle = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 1 },
}
