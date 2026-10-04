--!strict
-- The boomerang (the original's attack object #0x32, `sub_80CA2A8`): the
-- shared boomerang (@exelib/boomer/boomerang), BN6's kind and look: an
-- attack that hits navis, the wood spark.

local collision = require("../../rules/collision")
local common = require("@exelib/boomer/boomerang")
local sparks = require("../../lib/sparks")

local boomerang = {}

-- A boomerang variant.
boomerang.variant = common.variant

-- TomahawkCross Beast's tomahawk (the original's row 4).
boomerang.tomahawk = define.record("boomerang-variant", {
    speed = 0x8_0000,
    turn_speed = 314573,
    grass = false,
    tomahawk = true,
} :: BoomerangVariant)

boomerang.kind = common.make_kind {
    id = "boomer/boomerang",
    sprite = asset.sprite("boomerang"),
    tomahawk_sprite = asset.sprite("boomerang-tomahawk"),
    sound = asset.sound("boomerang"),
    is = collision.attack,
    hits = collision.hits_navis,
    spark = sparks.wood,
}

-- `sub_80CA4D8`: a boomerang of `owner`'s on (x, y), thrown as `throw`
-- says, with the damage word `damage`.
function boomerang.spawn(owner: Object, x: number, y: number, element: number, z: number, throw: BoomerangThrow, damage: number): Object?
    return common.spawn(boomerang.kind, owner, x, y, element, z, throw, damage)
end

-- `sub_80CA4F6`, instant chip effect 1: a chip's `instant` hook that
-- throws a boomerang of `variant`.
function boomerang.instant(variant: BoomerangVariant): (user: Object, spec: InstantChipSpec) -> ()
    return common.instant(boomerang.kind, variant)
end

return boomerang
