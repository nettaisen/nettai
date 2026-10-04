--!strict
-- CircGun's shot (the original's attack object #0x8A, `sub_80D67EC`): the
-- shared shot (@exelib/circgun/shot), BN6's kind and look.

local collision = require("../../rules/collision")
local common = require("@exelib/circgun/shot")
local effects = require("../../lib/effects")
local sparks = require("../../lib/sparks")

local shot = {}

shot.place = common.place
shot.place_on_floor = common.place_on_floor
shot.make_kind = common.make_kind
shot.spawn_kind = common.spawn_kind

-- BN6's look, which the gun's kind shares.
shot.look = {
    sprite = asset.sprite("sight"),
    sounds = {
        -- (The cursor steps with the sound EraseMan's mark has, the
        -- original's 0x10E.)
        step = asset.sound("erase-man"),
        mark = asset.sound("mark"),
        hit = asset.sound("col-army-2"),
    },
    spark = sparks.of_element(0),
    target = collision.hits_navis,
    flash = effects.flash,
    hits = {
        [0] = collision.thrown,
        [1] = define.collision { id = "circgun/other-hit", side0 = 0x80000488, side1 = 0x40000488, row_offset = 0x158 },
    },
} :: CircGunLook

shot.kind = shot.make_kind("circgun/shot", shot.look)

-- BN6's shot.
function shot.spawn(gun: Object, x: number, y: number, element: number, damage: number, ticks: number, look: number, slot: number): Object?
    return common.spawn_kind(shot.kind, gun, x, y, element, damage, ticks, look, slot)
end

return shot
