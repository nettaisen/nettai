--!strict
-- EXE4's boomerang (attack object 0x32, 0x080D3234: EXE5's code but for its constants, @exelib/boomer/boomerang),
-- EXE4's kind: its sprite (10-14), its sound (0xAB), an attack hitting navis with the wood spark (hit effect 4); like
-- EXE5's, it turns no panel to grass and has no tomahawk. Its variants are EXE5's rows (0x080D31EC, 12 bytes: the
-- speeds); the spawners' variant 2 (0x080D344E) throws it from the user's back column along row 3, 16 pixels up,
-- turning at the far column, its parameter word the chip's (+0x10: the row; the throw neither straight nor strong).

local collision = require("../../rules/collision")
local common = require("@exelib/boomer/boomerang")
local sparks = require("../../lib/sparks")

local boomerang = {}

local SPRITE = asset.sprite("boomerang")

boomerang.kind = common.make_kind {
    id = "boomer/boomerang",
    sprite = SPRITE,
    tomahawk_sprite = SPRITE,
    sound = asset.sound("boomerang"),
    is = collision.attack,
    hits = collision.hits_navis,
    spark = sparks.wood,
}

-- Row 5 of 0x080D31EC (the Boomers'): 8 pixels a tick along the rows, 4.8 along the column.
boomerang.boomer = common.variant { speed = 0x8_0000, turn_speed = 314573, grass = false }

-- 0x080D344E: a boomerang of `variant` from the user's back column (by its side: EXE4 has no flip), row 3, 16 pixels
-- up, with the attack's element and damage word.
function boomerang.spawner(variant: BoomerangVariant): (user: Object, element: number, damage: number) -> ()
    return function(user: Object, element: number, damage: number)
        local x = user.alliance * 5 + 1
        common.spawn(boomerang.kind, user, x, 3, element, 0x10_0000, { variant = variant }, damage)
    end
end

return boomerang
