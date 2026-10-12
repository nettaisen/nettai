--!strict
-- Magnum's gunner (EXE5's actor object #2, 0x080B8CA8): @exelib/magnum/gunner in EXE5's look: an actor wearing a player
-- user's overlay, its sounds (0x10E a step of the sight, 0xB4 the shot, 0x10F the blast, 0x97 a panel broken), firing
-- in animation 12, the sight of effect row 0x2C, and the sight sweeping on round the field (0x080B8EFC).

local collision = require("../../rules/collision")
local common = require("@exelib/magnum/gunner")
local sparks = require("../../lib/sparks")
local stand_in = require("@exelib/dimming/stand_in")

return common.make {
    id = "magnum/gunner",
    pool = "actor",
    wear = stand_in.overlay,
    step_sound = asset.sound("erase-man"),
    shot = asset.sound("hockey-puck"),
    blast = asset.sound("target-move"),
    crack = asset.sound("panel-crack"),
    fire_anim = 12,
    sight = new.effect { sprite = asset.sprite("sight"), anim = 0, palette = 0 },
    explosion = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0 },
    spark = sparks.heavy,
    is = collision.thrown,
    hits = collision.hits_navis,
    step = common.sweep,
}
