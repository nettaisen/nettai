--!strict
-- Boomer (BN5 chip 0x006): a boomerang flies from the user's back column along row 3, up the far column and back
-- along row 1: the shared boomerang (@exelib/boomer/boomerang, instant chip effect 1; BN5's the same code but that
-- it turns no panel to grass and has no tomahawk), with BN5's variant (0x080CAF50 row 5, its record's parameter:
-- BN6's Boomer's speeds), sprite (10-14), collision type and spark.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local boomerang = require("@exelib/boomer/boomerang")
local kind = require("./boomerang")
local collision = require("../../rules/collision")
local sparks = require("../../lib/sparks")

local VARIANT = boomerang.variant {
    speed = 0x8_0000,
    turn_speed = 314573,
    grass = false,
    sprite = asset.sprite("bn5:boomerang"),
    is = collision.attack,
    spark = sparks.wood,
}

return define.chip {
    id = "bn5:boomer",
    codes = { "A", "L", "V", "*" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 2,
    mb = 16,
    damage = 60,
    hit_param = 10,
    lockout = 30,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    -- (Strong: BN5's instant chip dispatch leaves 0x01000005 in the boomerang's parameters, its Param4 1.)
    instant = boomerang.instant(kind, VARIANT, true),
}
