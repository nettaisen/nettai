--!strict
-- The bullet (the original's attack object #0x12, `sub_80C6946`): the
-- shared bullet (@exelib/bullet/bullet), EXE6's kind and look. Its
-- variants (`bullet.variant { ... }`, rows of `byte_80C68D4`: region, hit
-- modifier, effect, status, sound) are its users': the Vulcans' and
-- Spreaders' shots, SpoutCross's charged shot, the patch card puffy's,
-- ColArmy's and ColForce's soldiers'. See docs/engine/shot-chips.md §4.1.

local collision = require("../rules/collision")
local common = require("@exelib/bullet")

local bullet = {}

-- A bullet variant.
bullet.variant = common.variant

-- A bullet kind (`id`) of `look`.
bullet.make_kind = common.make_kind

-- EXE6's look: the probe's self type (row 0x3D), the burst's (an attack)
-- and what both reach.
local EXE6: BulletLook = { probe = collision.probe, attack = collision.attack, target = collision.hits_navis }

bullet.kind = common.make_kind("bullet", EXE6)

-- `sub_80C6ADA`: a bullet of `variant` from `owner` on panel (x, y) at
-- height `z` (16.16), with `element` and the damage word. Of `kind`, EXE6's
-- by default.
function bullet.fire(owner: Object, x: number, y: number, z: number, variant: BulletVariant, element: number, damage: number, kind: Kind?): Object?
    return common.fire(kind or bullet.kind, owner, x, y, z, variant, element, damage)
end

-- The looks several users' variants share: the Vulcans' and the
-- soldiers' hit (shown once, at the bullet), and the splash with its
-- sound (SpoutCross's charged shot and the puffy's).
bullet.hit = new.effect { sprite = asset.sprite("vulcan-hit"), anim = 0, palette = 0 }
bullet.splash = new.effect { sprite = asset.sprite("spout-splash"), anim = 0, palette = 0 }
bullet.splash_sound = asset.sound("spout-ball")

return bullet
