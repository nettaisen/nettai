--!strict
-- SearchMan's shot, EXE4's (attack object #0x57, 0x080D87B0): @exelib/serchman/shot with EXE4's: on the ground
-- (0x080D87DE), its miss's spark effect row 8 (hit-sparks, animation 0), the rows 25 and 29 (curse and row 0x1D) its
-- Param1 names, sparks 0 and 9.

local collision = require("../../rules/collision")
local common = require("@exelib/serchman/shot")
local sparks = require("../../lib/sparks")

return common.make {
    id = "serchman/shot",
    z = 0,
    miss = new.effect { sprite = asset.sprite("hit-sparks"), anim = 0, palette = 0 },
    plain_type = collision.curse,
    powered_type = collision.collision_1d,
    target = collision.hits_navis,
    plain_spark = sparks.plain,
    powered_spark = sparks.chip_destruction,
}
