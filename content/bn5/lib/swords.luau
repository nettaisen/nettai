--!strict
-- BN5's swords' parts, for BN6's slash (@bn6/lib/swords/slash, the original's action 0x13; BN5's the same code
-- but for a step sword's afterimage and the elemental swords' slash palettes, which these swords haven't): the
-- blade (attachment row 3, as BN6's: the same look), the slash's effects (BN5's effects 0x16 to 0x18 are
-- BN6's: the same rows of 0x080DFC94, by the swords' table 0x080EC028), the swing's sound, and BN5's
-- collision types. BN5's blade animation and attachment row go by its souls (0x080EC038 and 0x080EC082,
-- `sub_80EBAE8`'s and `sub_80EBB34`'s counterparts): without one, BN6's base MegaMan's, the only case here until
-- the souls come (bn5-map.md §15). BN5's swing also calls 0x080E9FD2 where BN6's calls `nullsub_12`: with battle
-- flag 0x40 (`battle.per_player_gauges()`, never set in a netbattle: bn5-map.md §3) it spawns effect object 0x83
-- on the panel ahead, which isn't ported. And BN5's sonic boom (attack object #0x58, BN6's the same number:
-- 0x080D0E64, BN6's `sub_80CF810`, the same code but for its look): at the floor (BN6's 16 pixels up), its palette
-- 9 going through (BN6's 0xB), else by its whole element byte (BN6's its primary element) from 0x080D0EF4, a BN5
-- slash.

local attachment = require("@bn6/objects/attachment/attachment")
local collision = require("../rules/collision")
local sonic_boom = require("@bn6/lib/swords/sonic_boom")

local swords = {}

local SLASH = asset.sprite("bn5:sword-slash")

swords.blade = attachment.look { sprite = asset.sprite("bn5:sword"), palette = 0 }
swords.slashes_navis = { is = collision.slash, hits = collision.hits_navis }
swords.effects = {
    -- Sword's, over the panel ahead (effect 0x18).
    slash = define.effect { sprite = SLASH, anim = 2, palette = 0 },
    -- Over a column: WideSwrd's (0x16).
    wide = define.effect { sprite = SLASH, anim = 0, palette = 0 },
    -- Over two panels: LongSwrd's (0x17).
    long = define.effect { sprite = SLASH, anim = 1, palette = 0 },
}
swords.swing = asset.sound("bn5:sword-swing")
swords.sonic_boom = sonic_boom.make_kind("bn5:sonic-boom", {
    sprite = SLASH,
    z = 0,
    piercing_palette = 9,
    palettes = { [0] = 0, 2, 5, 4, 8, 0, 0, 0 },
    element_mask = 0xFF,
    is = collision.slash,
})

return swords
