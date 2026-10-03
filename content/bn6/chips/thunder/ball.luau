--!strict
-- The thunder ball (the original's attack object #0x2A, `sub_80C93CC`):
-- the shared ball (@common/thunder/ball), BN6's kind and look.

local collision = require("../../rules/collision")
local common = require("@common/thunder/ball")
local sparks = require("../../lib/sparks")

local ball = {}

-- A thunder ball kind (`id`) of `look`.
ball.make_kind = common.make_kind

-- BN6's look: one sprite, its animations small and big (fast, or
-- DarkThnd's 12 panels; +1 bursting); an attack hitting navis with
-- modifier 1 and the elec spark; its sound, DarkThnd's own.
local SPRITE = asset.sprite("bn6:thunder-ball")
local BN6: ThunderBallLook = {
    sprite = SPRITE,
    small_anim = 0,
    big_sprite = SPRITE,
    big_anim = 2,
    is = collision.attack,
    target = collision.hits_navis,
    hit_mod = 1,
    spark = sparks.elec,
    sound = asset.sound("bn6:erase-man-2"),
    dark_sound = asset.sound("bn6:elec-man"),
}

ball.kind = common.make_kind("bn6:thunder/ball", BN6)

-- `sub_80C954C`: a thunder ball for `owner` on (x, y) with the damage word
-- `damage`. (A game's own kind of it: `kind`.)
function ball.spawn(owner: Object, x: number, y: number, element: number, damage: number, spec: ThunderBall, kind: Kind?): Object?
    return common.spawn(kind or ball.kind, owner, x, y, element, damage, spec)
end

return ball
