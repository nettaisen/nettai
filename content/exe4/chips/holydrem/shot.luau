--!strict
-- HolyDrem's shot (EXE4's attack object #0x83, 0x080DD9EC: @exelib/holydrem/shot) in EXE4's look: sheet 0c-3e, the
-- energy burst's sound (0xBD) and collision rows 0x17 and 5, the plain spark; its spawn (0x080DDB14) takes its owner's
-- side alone.

local collision = require("../../rules/collision")
local common = require("@exelib/holydrem/shot")
local sparks = require("../../lib/sparks")

return common.make {
    id = "holydrem/shot",
    sprite = asset.sprite("holy-dream-shot"),
    sound = asset.sound("energy-burst"),
    is = collision.thrown_piercing_break,
    hits = collision.hits_navis,
    spark = sparks.plain,
    side_only = true,
}
