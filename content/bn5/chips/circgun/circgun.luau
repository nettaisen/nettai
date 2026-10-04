--!strict
-- BN5's CircGun kinds, CircGun's and DarkCirc's (the dimming handler's subtype 42; effect object #0x5E, attack
-- objects #0x89 and #0x8A): the shared CircGun (@common/circgun, BN6's code) with BN5's look: its sight (10-22),
-- sounds, spark and the shots' hit types (BN5's rows 0xA and 0x2B), and its gun and shots on their panels' centers
-- at height 0 (0x080D76F0, where BN6's go 8 pixels down and up). What differs between the chips is the controller's
-- variant (`circgun.variant`: period, shots and look).

local collision = require("../../rules/collision")
local controller = require("@common/circgun/controller")
local effects = require("../../lib/effects")
local gun = require("@common/circgun/gun")
local shot = require("@common/circgun/shot")
local sparks = require("../../lib/sparks")

local circgun = {}

local LOOK: CircGunLook = {
    sprite = asset.sprite("bn5:sight"),
    sounds = {
        step = asset.sound("bn5:erase-man"),
        mark = asset.sound("bn5:mark"),
        hit = asset.sound("bn5:col-army-2"),
    },
    hits = { [0] = collision.thrown, [1] = collision.collision_2b },
    -- (What the hit reaches and its flash BN6's.)
    target = collision.hits_navis,
    flash = effects.flash,
    spark = sparks.plain,
    on_floor = true,
}

local SHOT = shot.make_kind("bn5:circgun/shot", LOOK)
local GUN = gun.make_kind("bn5:circgun/gun", LOOK, SHOT)
local CONTROLLER = controller.make("bn5:circgun/controller", GUN)

-- A controller variant.
circgun.variant = controller.variant

-- A chip's `dimming` hook, of controller variant `v`.
circgun.hook = CONTROLLER.hook

return circgun
