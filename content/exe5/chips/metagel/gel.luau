--!strict
-- MetaGel's gel (EXE5's attack object #0x45, 0x080CE02C): @exelib/metagel/gel in EXE5's look: its sprite (0c-26), its
-- sounds (0x105 as it drops, 0x121 as it lands), the aqua spark, EXE5's rows 0xA and 5, the puff of effect row 0x14,
-- and the panel taken where the other side may lose it (0x0800BBCC).

local collision = require("../../rules/collision")
local common = require("@exelib/metagel/gel")
local sparks = require("../../lib/sparks")

return common.make {
    id = "metagel/gel",
    sprite = asset.sprite("meta-gel"),
    drop = asset.sound("drop"),
    land = asset.sound("gel-land"),
    spark = sparks.aqua,
    is = collision.thrown,
    hits = collision.hits_navis,
    takes = common.can_take,
    puff = new.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 },
}
