--!strict
-- EXE5's bullet (attack object 0x12, 0x080C6B98): the shared bullet (@exelib/bullet/bullet, EXE6's code) but for its
-- collision types. It probes as EXE5's row 0 (nothing; EXE6's row 0x3D) and bursts as EXE5's rows 4 and 5 (0x080C6BEC,
-- 0x080C6D10). Its burst sets no status (EXE6's `sub_80C6AB8` gives the variant's): EXE5's variants have none. EXE5 sets
-- the collision's element to the whole element byte where EXE6 splits it (no secondary elements: the same for EXE5's
-- elements). Its variants are EXE5's rows (0x080C6B20): the Vulcans' row 0xC and SuprVulc's 0x10 are EXE6's (their hit
-- effects EXE6's: SuprVulc's EXE5's effect 0x3D, EXE6's 0x6A, the same row of the effect table).

local collision = require("../rules/collision")
local common = require("@exelib/bullet")
local regions = require("@exelib/regions")

local bullet = {}

bullet.kind = common.make_kind("bullet", {
    probe = collision.nothing,
    attack = collision.attack,
    target = collision.hits_navis,
})

bullet.variants = {
    -- The Vulcans' (row 0xC: EXE6's): the panel found and the one behind it, staggering, its hit shown once at the
    -- bullet.
    vulcan = common.variant {
        region = regions.long,
        hit_mod = 1,
        effect = define.effect { sprite = asset.sprite("vulcan-hit"), anim = 0, palette = 0 },
        single = true,
    },
    -- The patch cards' HeatS's (row 8, its effect 0x22 and sound EXE6's): the panel found and the one behind it.
    heat = common.variant {
        region = regions.long,
        hit_mod = 0,
        effect = define.effect { sprite = asset.sprite("shell-burst"), anim = 0, palette = 0 },
        sound = asset.sound("target-move"),
    },
    -- The patch cards' BubSht's (row 4, its effect 0x1F and sound EXE6's): the panel found and the one behind it.
    bubble = common.variant {
        region = regions.long,
        hit_mod = 0,
        effect = define.effect { sprite = asset.sprite("spout-splash"), anim = 0, palette = 0 },
        sound = asset.sound("spout-ball"),
    },
    -- The Spreader's (row 3: EXE6's with EXE5's flash, effect 0x21, and sound): the 3x3 around the panel found.
    spreader = common.variant {
        region = regions.square,
        hit_mod = 0,
        effect = define.effect { sprite = asset.sprite("pink-flash"), anim = 0, palette = 0 },
        sound = asset.sound("h-burst"),
    },
    -- The InfVulcs' (row 0xE: EXE6's but for its hit, EXE5's effect 0x3E, the Vulcans' in palette 2; EXE6's a glint).
    infinite = common.variant {
        region = regions.long,
        hit_mod = 1,
        effect = define.effect { sprite = asset.sprite("vulcan-hit"), anim = 0, palette = 2 },
        single = true,
    },
    -- SuprVulc's (row 0x10: EXE6's): the same, its hit brighter.
    super_vulcan = common.variant {
        region = regions.long,
        hit_mod = 1,
        effect = define.effect { sprite = asset.sprite("vulcan-hit"), anim = 0, palette = 1 },
        single = true,
    },
    -- NapalmSoul's charged shot's (rows 0x11 and 0x12, the last one's; EXE6's ColArmy's and ColForce's with their
    -- statuses): the panel found and the one behind it, staggering (the last flinching), its hit shown once at the
    -- bullet (effect 0x58: EXE5's own vulcan hit, its animation 1).
    napalm = common.variant {
        region = regions.long,
        hit_mod = 1,
        effect = define.effect { sprite = asset.sprite("vulcan-hit"), anim = 1, palette = 0 },
        single = true,
    },
    napalm_last = common.variant {
        region = regions.long,
        hit_mod = 3,
        effect = define.effect { sprite = asset.sprite("vulcan-hit"), anim = 1, palette = 0 },
        single = true,
    },
}

return bullet
