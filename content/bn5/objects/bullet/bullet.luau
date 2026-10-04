--!strict
-- BN5's bullet (attack object 0x12, 0x080C6B98): the shared bullet (@common/bullet/bullet, BN6's code) but for its
-- collision types. It probes as BN5's row 0 (nothing; BN6's row 0x3D) and bursts as BN5's rows 4 and 5 (0x080C6BEC,
-- 0x080C6D10). Its burst sets no status (BN6's `sub_80C6AB8` gives the variant's): BN5's variants have none. BN5 sets
-- the collision's element to the whole element byte where BN6 splits it (no secondary elements: the same for BN5's
-- elements). Its variants are BN5's rows (0x080C6B20): the Vulcans' row 0xC and SuprVulc's 0x10 are BN6's (their hit
-- effects BN6's: SuprVulc's BN5's effect 0x3D, BN6's 0x6A, the same row of the effect table).

local collision = require("../../rules/collision")
local common = require("@common/bullet/bullet")
local regions = require("@common/regions")

local bullet = {}

bullet.kind = common.make_kind("bn5:bullet", {
    probe = collision.nothing,
    attack = collision.attack,
    target = collision.hits_navis,
})

bullet.variants = {
    -- The Vulcans' (row 0xC: BN6's): the panel found and the one behind it, staggering, its hit shown once at the
    -- bullet.
    vulcan = common.variant {
        region = regions.long,
        hit_mod = 1,
        effect = define.effect { sprite = asset.sprite("bn6:vulcan-hit"), anim = 0, palette = 0 },
        single = true,
    },
    -- The patch cards' HeatS's (row 8: BN6's, its effect 0x22 and sound BN6's): the panel found and the one behind it.
    heat = common.variant {
        region = regions.long,
        hit_mod = 0,
        effect = define.effect { sprite = asset.sprite("bn5:shell-burst"), anim = 0, palette = 0 },
        sound = asset.sound("bn5:target-move"),
    },
    -- The patch cards' BubSht's (row 4: BN6's, its effect 0x1F and sound BN6's): the panel found and the one behind it.
    bubble = common.variant {
        region = regions.long,
        hit_mod = 0,
        effect = define.effect { sprite = asset.sprite("bn5:spout-splash"), anim = 0, palette = 0 },
        sound = asset.sound("bn5:spout-ball"),
    },
    -- The Spreader's (row 3: BN6's with BN5's flash, effect 0x21, and sound): the 3x3 around the panel found.
    spreader = common.variant {
        region = regions.square,
        hit_mod = 0,
        effect = define.effect { sprite = asset.sprite("bn5:pink-flash"), anim = 0, palette = 0 },
        sound = asset.sound("bn5:h-burst"),
    },
    -- The InfVulcs' (row 0xE: BN6's but for its hit, BN5's effect 0x3E, the Vulcans' in palette 2; BN6's a glint).
    infinite = common.variant {
        region = regions.long,
        hit_mod = 1,
        effect = define.effect { sprite = asset.sprite("bn6:vulcan-hit"), anim = 0, palette = 2 },
        single = true,
    },
    -- SuprVulc's (row 0x10: BN6's): the same, its hit brighter.
    super_vulcan = common.variant {
        region = regions.long,
        hit_mod = 1,
        effect = define.effect { sprite = asset.sprite("bn6:vulcan-hit"), anim = 0, palette = 1 },
        single = true,
    },
    -- NapalmSoul's charged shot's (rows 0x11 and 0x12, the last one's; BN6's ColArmy's and ColForce's with their
    -- statuses): the panel found and the one behind it, staggering (the last flinching), its hit shown once at the
    -- bullet (effect 0x58: BN5's own vulcan hit, its animation 1).
    napalm = common.variant {
        region = regions.long,
        hit_mod = 1,
        effect = define.effect { sprite = asset.sprite("bn5:vulcan-hit"), anim = 1, palette = 0 },
        single = true,
    },
    napalm_last = common.variant {
        region = regions.long,
        hit_mod = 3,
        effect = define.effect { sprite = asset.sprite("bn5:vulcan-hit"), anim = 1, palette = 0 },
        single = true,
    },
}

return bullet
