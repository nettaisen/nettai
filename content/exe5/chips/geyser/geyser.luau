--!strict
-- EXE5's Geyser (attack objects #0x42, 0x080CD992, and #0x43, 0x080CDC2C; thrown by action 0x12's subtype 5, 0x080CDC08):
-- the shared one (@exelib/geyser/geyser, EXE5's code, which EXE4's Geyser shares) with EXE5's look: the bomb's sprite
-- (0x080CD990's row by its kind, Param1: the chip's kind 0, the bomb) and the geyser's (0c-25), effect 0x1F (a splash)
-- and its sound, the eruption's sound, aqua hits (an attack reaching navis, the aqua spark), its water's height the
-- battle state's address.

local collision = require("../../rules/collision")
local common = require("@exelib/geyser/geyser")
local sparks = require("../../lib/sparks")
local trajectory = require("../../lib/trajectory")

local made = common.make({
    id = "geyser/bomb",
    water_id = "geyser/water",
    bomb = asset.sprite("bomb"),
    geyser = asset.sprite("geyser"),
    splash = new.effect { sprite = asset.sprite("spout-splash"), anim = 0, palette = 0 },
    splash_sound = asset.sound("spout-ball"),
    erupt = asset.sound("geyser"),
    attack = collision.attack,
    hits = collision.hits_navis,
    spark = sparks.aqua,
    -- The battle state's address (0x08006038: the battle-over test's r3, which the water's spawn keeps as its height).
    water_height = 0x0203_4A90,
}, trajectory)

return { kind = made.kind, water = made.water, thrower = made.thrower }
