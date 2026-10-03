--!strict
-- BN5's bullet (attack object 0x12, 0x080C6B98): BN6's (@bn6/objects/bullet) but for its collision types. It
-- probes as BN5's row 0 (nothing; BN6's row 0x3D) and bursts as BN5's rows 4 and 5 (0x080C6BEC, 0x080C6D10). Its
-- burst sets no status (BN6's `sub_80C6AB8` gives the variant's): BN5's variants have none. BN5 sets the
-- collision's element to the whole element byte where BN6 splits it (no secondary elements: the same for BN5's
-- elements). Its variants are BN5's rows (0x080C6B20): the Vulcans' row 0xC is BN6's.

local bn6_bullet = require("@bn6/objects/bullet/bullet")
local collision = require("../../rules/collision")

local bullet = {}

bullet.kind = bn6_bullet.make_kind("bn5:bullet", {
    probe = collision.nothing,
    attack = collision.attack,
    target = collision.hits_navis,
})

bullet.variants = {
    -- The Vulcans' (row 0xC: BN6's): the panel found and the one behind it, staggering.
    vulcan = bn6_bullet.variants.vulcan,
    -- SuprVulc's (row 0x10: BN6's, its effect BN5's 0x3D, BN6's 0x6A: the same row of the effect table).
    super_vulcan = bn6_bullet.variants.super_vulcan,
}

return bullet
