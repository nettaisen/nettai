--!strict
-- EXE5's VDoll curse (effect object #0x4E, 0x080E6110): the shared one (@exelib/vdoll/curse, EXE6's table, EXE5's own
-- marking and hit) in EXE5's look: its telop EXE5's Curse (chip 0x174, ../curse/chip); the mark's sound (0x08009620,
-- EXE6's `sub_800AE90`: 0x91); its hit EXE5's row 0x17, a thrown piercing break against navis, spark 6 (EXE6's the
-- curse row, hit modifier 3); EXE5's sparkles (./sparkles) of effect 0x24 (EXE5's table, 0x080DFC94: sheet 14-00 in
-- palette 1).

local collision = require("../../rules/collision")
local common = require("@exelib/vdoll/curse")
local curse_chip = require("../curse").curse
local sparkles = require("./sparkles")
local sparks = require("../../lib/sparks")

return common.make {
    id = "vdoll/curse",
    chip = curse_chip,
    mark_sound = asset.sound("err-select-91"),
    spark = sparks.heavy,
    is = collision.thrown_piercing_break,
    hits = collision.hits_navis,
    sparkles = sparkles,
    sparkle = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 1 },
}
