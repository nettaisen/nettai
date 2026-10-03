--!strict
-- BN5's projectile (attack object #0, BN6's code: @common/projectile/projectile), BN5's kind; a bursting variant's
-- burst BN6's (its sound, effect and collision types). Its variants are BN5's (./variants).

local bn6_collision = require("@bn6/rules/collision")
local common = require("@common/projectile/projectile")
local effects = require("@bn6/lib/effects")

local projectile = {}

-- A projectile variant.
projectile.variant = common.variant

projectile.kind = common.make_kind {
    id = "bn5:projectile",
    burst_sound = asset.sound("bn6:burst"),
    burst_effect = effects.explosion,
    burst_target = bn6_collision.hits_navis,
    burst_self_type = bn6_collision.attack,
}

-- A projectile of `shot.variant` from `owner` on panel (x, y), with the owner's side and flip.
function projectile.spawn(owner: Object, x: number, y: number, shot: ProjectileShot): Object?
    return common.spawn(projectile.kind, owner, x, y, shot)
end

return projectile
