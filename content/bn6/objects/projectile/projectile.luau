--!strict
-- The projectile (the original's attack object #0, `sub_80C4E58`): the
-- shared projectile (@exelib/projectile/projectile), BN6's kind and burst.
-- What a shot does is its variant (a record: the shared ones in ./variants,
-- a chip's own with the chip).

local collision = require("../../rules/collision")
local common = require("@exelib/projectile/projectile")
local effects = require("../../lib/effects")

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

-- `sub_80C4FFE`: a projectile of `shot.variant` from `owner` on panel (x,
-- y), with the owner's side and flip; nil if the attack pool is full.
function projectile.spawn(owner: Object, x: number, y: number, shot: ProjectileShot): Object?
    return common.spawn(projectile.kind, owner, x, y, shot)
end

return projectile
