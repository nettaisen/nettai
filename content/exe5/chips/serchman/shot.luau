--!strict
-- SearchMan's shot, EXE5's (attack object #0x57, 0x080D0D78): @exelib/serchman/shot with EXE5's: 16 pixels up, its
-- miss's spark effect 7 (hit-sparks, animation 0), the rows 25 and 29 (curse and row 0x1D) its Param1 names.

local collision = require("../../rules/collision")
local common = require("@exelib/serchman/shot")
local sparks = require("../../lib/sparks")

return common.make {
    id = "serchman/shot",
    z = 0x10_0000,
    miss = new.effect { sprite = asset.sprite("hit-sparks"), anim = 0, palette = 0 },
    plain_type = collision.curse,
    powered_type = collision.collision_1d,
    target = collision.hits_navis,
    plain_spark = sparks.plain,
    powered_spark = sparks.chip_destruction,
}
