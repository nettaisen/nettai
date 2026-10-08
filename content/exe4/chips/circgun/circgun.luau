--!strict
-- EXE4's CircGun kinds (the dimming chips' variant 0x33: effect object #0x5E, 0x080E8AFC; attack objects #0x89,
-- 0x080DE610, and #0x8A, 0x080DE888): the shared CircGun (@exelib/circgun, EXE6's code) with EXE4's look: its sight
-- (10-22, SearchMan's scope's sheet), sounds (a step 0x128, the mark 0x12F, the hit 0xBB), the hit's flash (effect
-- row 0x27, 14-04), spark 0 and hit types (thrown, at navis), its gun and shots on their panels' centers at height 0;
-- its gun turning back along a row on a column with any of the user's side's panels (0x080DE7AA); and its controller
-- (0x080E8B40) setting the gun whatever the far column holds, its spawn (0x080E8B84) taking the user's side alone.

local collision = require("../../rules/collision")
local controller = require("@exelib/circgun/controller")
local gun = require("@exelib/circgun/gun")
local shot = require("@exelib/circgun/shot")
local sparks = require("../../lib/sparks")

local circgun = {}

local LOOK: CircGunLook = {
    sprite = asset.sprite("search-scope"),
    sounds = {
        step = asset.sound("scope-move"),
        mark = asset.sound("mark"),
        hit = asset.sound("vulcan"),
    },
    target = collision.hits_navis,
    flash = new.effect { sprite = asset.sprite("pink-flash"), anim = 0, palette = 0 },
    spark = sparks.plain,
    on_floor = true,
    turns_on_own_column = true,
}

local SHOT = shot.make_kind("circgun/shot", LOOK)
local GUN = gun.make_kind("circgun/gun", LOOK, SHOT)
local CONTROLLER = controller.make("circgun/controller", GUN, { side_only = true, always_sets_gun = true })

-- A controller variant.
circgun.variant = controller.variant

-- A chip's `dimming` hook, of controller variant `v`.
circgun.hook = CONTROLLER.hook

return circgun
