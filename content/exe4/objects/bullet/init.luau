--!strict
-- EXE4's bullet (attack object 0x12, 0x080CF420): the shared bullet (@exelib/bullet, EXE6's code) on EXE4's
-- collision rows. It probes as row 0 (nothing) and bursts as rows 4 and 5 (0x080CF46C, 0x080CF580); its burst sets
-- no status; a single burst's effect shows unturned (0x080CF55E passes no flip). Each chip states its own variant
-- (`bullet.variant`), the row of 0x080CF3DC its parameter +0x10 names (the region, the hit modifier, the effect row of
-- 0x080E1698 and the sound; rows 12, 14 and 16 show their effect once at the bullet, 0x080CF538, the others over the
-- region).

local collision = require("../rules/collision")
local common = require("@exelib/bullet")

local bullet = {}

bullet.kind = common.make_kind("bullet", {
    probe = collision.nothing,
    attack = collision.attack,
    target = collision.hits_navis,
    single_unturned = true,
})

-- A bullet variant (a row of 0x080CF3DC): its region, hit modifier, effect, sound and whether its effect shows once at
-- the bullet.
bullet.variant = common.variant

-- 0x080CF596: a bullet of `variant` from `owner` on panel (x, y) at height `z`, with `element` and the damage word.
function bullet.fire(owner: Object, x: number, y: number, z: number, variant: BulletVariant, element: number, damage: number): Object?
    return common.fire(bullet.kind, owner, x, y, z, variant, element, damage)
end

return bullet
