--!strict
-- BN5's bullet (attack object 0x12, 0x080C6B98): BN6's (@bn6/objects/bullet) but for its collision types. It
-- probes as BN5's row 0 (nothing; BN6's row 0x3D) and bursts as BN5's rows 4 and 5 (0x080C6BEC, 0x080C6D10). Its
-- burst sets no status (BN6's `sub_80C6AB8` gives the variant's): BN5's variants have none. BN5 sets the
-- collision's element to the whole element byte where BN6 splits it (no secondary elements: the same for BN5's
-- elements). Its variants are BN5's rows (0x080C6B20): the Vulcans' row 0xC is BN6's.

local bn6_bullet = require("@bn6/objects/bullet/bullet")
local collision = require("../../rules/collision")
local regions = require("@bn6/lib/regions")

local bullet = {}

bullet.kind = bn6_bullet.make_kind("bn5:bullet", {
    probe = collision.nothing,
    attack = collision.attack,
    target = collision.hits_navis,
})

bullet.variants = {
    -- The Vulcans' (row 0xC: BN6's): the panel found and the one behind it, staggering.
    vulcan = bn6_bullet.variants.vulcan,
    -- The Spreader's (row 3: BN6's with BN5's flash, effect 0x21, and sound): the 3x3 around the panel found.
    spreader = bn6_bullet.variant {
        region = regions.square,
        hit_mod = 0,
        effect = define.effect { sprite = asset.sprite("bn5:pink-flash"), anim = 0, palette = 0 },
        sound = asset.sound("bn5:h-burst"),
    },
}

return bullet
