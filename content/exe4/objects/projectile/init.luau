--!strict
-- EXE4's projectile (attack object #0: init 0x080CD354, tick 0x080CD3D4, spawn 0x080CD45A; EXE5's 0x080C48B0 and on,
-- @exelib/projectile/projectile, nearly): a variant is a row of 0x080CD2A8 (9 bytes: what it is and hits, the hit
-- modifier, the element, the spark, the sprite or none, its animation), the rows EXE4's actions fire in ./variants.
-- EXE4's has no burst, no climb and no panel hit; its rows 10 and 11 set a status (0x08013212) the variants don't
-- state yet. Where its code differs from EXE5's is to read (docs/design/exe4-map.md §18).

local common = require("@exelib/projectile/projectile")

local projectile = {}

-- A projectile variant.
projectile.variant = common.variant

projectile.kind = common.make_kind { id = "projectile" }

-- A projectile of `shot.variant` from `owner` on panel (x, y), with the owner's side and flip.
function projectile.spawn(owner: Object, x: number, y: number, shot: ProjectileShot): Object?
    return common.spawn(projectile.kind, owner, x, y, shot)
end

return projectile
