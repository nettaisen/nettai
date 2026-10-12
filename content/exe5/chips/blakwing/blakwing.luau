--!strict
-- EXE5's BlakWing (the dimming handler's subtype 32): the shared BlakWing (@exelib/blakwing, EXE4's too), EXE5's
-- parts: sheet 0C-42, the puff (effect row 20), the start's and the flap's sounds (0x112, 0x130), its wings an
-- attack of type 0x480 hitting navis (row 0x25) with the plain spark, its perches listed as found from the enemy
-- area's front column (0x080D633E).

local collision = require("../../rules/collision")
local common = require("@exelib/blakwing")
local sparks = require("../../lib/sparks")

local blakwing = {}

-- 0x080E69CC: the chip's `dimming` hook.
blakwing.hook = common.make {
    controller_id = "blakwing/controller",
    flock_id = "blakwing/flock",
    perch_id = "blakwing/perch",
    wing_id = "blakwing/wing",
    sprite = asset.sprite("black-wing"),
    puff = new.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 },
    sounds = { start = asset.sound("place"), flap = asset.sound("black-wing-flap") },
    is = collision.attack_480,
    hits = collision.hits_navis,
    spark = sparks.plain,
}

return blakwing
