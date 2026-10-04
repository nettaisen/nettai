--!strict
-- BN5's projectile (attack object #0, BN6's code: @exelib/projectile/projectile), BN5's kind; a bursting variant's
-- burst BN6's (its sound, effect and collision types). Its variants are BN5's (./variants).

local collision = require("../rules/collision")
local common = require("@exelib/projectile/projectile")
local effects = require("../lib/effects")

local projectile = {}

-- A projectile variant.
projectile.variant = common.variant

projectile.kind = common.make_kind {
    id = "projectile",
    burst_sound = asset.sound("burst"),
    burst_effect = effects.explosion,
    burst_target = collision.hits_navis,
    burst_self_type = collision.attack,
}

-- A projectile of `shot.variant` from `owner` on panel (x, y), with the owner's side and flip.
function projectile.spawn(owner: Object, x: number, y: number, shot: ProjectileShot): Object?
    return common.spawn(projectile.kind, owner, x, y, shot)
end

return projectile
