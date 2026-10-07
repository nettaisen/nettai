--!strict
-- EXE4's bullet (attack object 0x12, 0x080CF420): the shared bullet (@exelib/bullet, EXE6's code) on EXE4's
-- collision rows. It probes as row 0 (nothing) and bursts as rows 4 and 5 (0x080CF46C, 0x080CF580); its burst sets
-- no status; a single burst's effect shows unturned (0x080CF55E passes no flip). Its variants are EXE4's rows
-- (0x080CF3DC: the region, the hit modifier, the effect row of 0x080E1698 and the sound), which the chip's parameter
-- +0x10 names; rows 12, 14 and 16 show their effect once at the bullet (0x080CF538), the others over the region.

local collision = require("../rules/collision")
local common = require("@exelib/bullet")
local regions = require("@exelib/regions")

local bullet = {}

bullet.kind = common.make_kind("bullet", {
    probe = collision.nothing,
    attack = collision.attack,
    target = collision.hits_navis,
    single_unturned = true,
})

local PINK_FLASH, VULCAN_HIT = asset.sprite("pink-flash"), asset.sprite("vulcan-hit")
local BURST = new.effect { sprite = PINK_FLASH, anim = 0, palette = 0 }
local BUBBLE = new.effect { sprite = asset.sprite("bubble-burst"), anim = 0, palette = 0 }
local HEAT = new.effect { sprite = asset.sprite("heat-burst"), anim = 0, palette = 0 }
local BURST_SOUND, BUBBLE_SOUND, HEAT_SOUND = asset.sound("spreader"), asset.sound("bubble-burst"), asset.sound("heat-burst")

local function row(region: Region, hit_mod: number, effect: Effect, sound: Sound?, single: boolean?): BulletVariant
    return common.variant { region = region, hit_mod = hit_mod, effect = effect, sound = sound, single = single }
end

bullet.variants = {
    -- Rows 0 to 3: a burst (effect row 0x27) over the panel found and the one behind it, a V ahead of it, its column,
    -- the 3x3 around it (Spreader's, row 3); sound 0x8D.
    burst_long = row(regions.long, 0, BURST, BURST_SOUND),
    burst_fork = row(regions.fork, 0, BURST, BURST_SOUND),
    burst_wide = row(regions.wide, 0, BURST, BURST_SOUND),
    burst_square = row(regions.square, 0, BURST, BURST_SOUND),
    -- Rows 4 to 7: the Bubblers' (effect row 0x25, sound 0x9F): Bubbler's, Bub-V's, BublSide's, BubSprd's.
    bubble_long = row(regions.long, 0, BUBBLE, BUBBLE_SOUND),
    bubble_fork = row(regions.fork, 0, BUBBLE, BUBBLE_SOUND),
    bubble_wide = row(regions.wide, 0, BUBBLE, BUBBLE_SOUND),
    bubble_square = row(regions.square, 0, BUBBLE, BUBBLE_SOUND),
    -- Rows 8 to 11: the HeatShots' (effect row 0x29, sound 0xBC): HeatShot's, Heat-V's, HeatSide's, HeatSprd's.
    heat_long = row(regions.long, 0, HEAT, HEAT_SOUND),
    heat_fork = row(regions.fork, 0, HEAT, HEAT_SOUND),
    heat_wide = row(regions.wide, 0, HEAT, HEAT_SOUND),
    heat_square = row(regions.square, 0, HEAT, HEAT_SOUND),
    -- Row 12: the Vulcans' (effect row 0x26), the panel found and the one behind it, its hit shown once.
    vulcan = row(regions.long, 1, new.effect { sprite = VULCAN_HIT, anim = 0, palette = 0 }, nil, true),
    -- Row 13: a bubble over the panel found and the one behind it, modifier 3.
    bubble_heavy = row(regions.long, 3, BUBBLE, BUBBLE_SOUND),
    -- Row 14: DrkVulcn's (effect row 0x6A: the Vulcans' hit in palette 2).
    dark_vulcan = row(regions.long, 1, new.effect { sprite = VULCAN_HIT, anim = 0, palette = 2 }, nil, true),
    -- Row 15: DrkSpred's (effect row 0x64: the burst in palette 1), the 3x3 around, modifier 1.
    dark_burst = row(regions.square, 1, new.effect { sprite = PINK_FLASH, anim = 0, palette = 1 }, BURST_SOUND),
    -- Row 16: SuprVulc's (effect row 0x69: the Vulcans' hit in palette 1).
    super_vulcan = row(regions.long, 1, new.effect { sprite = VULCAN_HIT, anim = 0, palette = 1 }, nil, true),
}

-- 0x080CF596: a bullet of `variant` from `owner` on panel (x, y) at height `z`, with `element` and the damage word.
function bullet.fire(owner: Object, x: number, y: number, z: number, variant: BulletVariant, element: number, damage: number): Object?
    return common.fire(bullet.kind, owner, x, y, z, variant, element, damage)
end

return bullet
