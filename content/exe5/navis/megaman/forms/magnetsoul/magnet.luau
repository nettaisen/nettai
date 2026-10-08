--!strict
-- MagnetSoul's magnet field (EXE5's attack object 0x75, 0x080D4B60, where EXE6's is AirRaid's plane): @exelib/magbolt/
-- magnet (EXE4's MagBolts' field, the same code) with EXE5's look: an attack against navis with its element's spark,
-- paralyzing for 90 ticks (status byte 0x10) when its parameter says; its spawn (0x080D4C32) stores it in its owner's
-- first related object, which he clears to let go of it.

local collision = require("../../../../rules/collision")
local common = require("@exelib/magbolt/magnet")
local sparks = require("../../../../lib/sparks")
local status = require("../../../../rules/status")

local magnet = {}

magnet.spawn = common.make {
    id = "magnetsoul/magnet",
    attack = collision.attack,
    hits = collision.hits_navis,
    spark_of = sparks.of_element,
    paralysis = status.paralyze_90,
    watches = false,
}

return magnet
