--!strict
-- BN5's instruments (attack object #0x78) and their dimming controller (effect object #0x4A): the shared code
-- (@common/instruments: BN6's) but for their look: the sprite (04-08), 100 HP (0x080D5224, BN6's 60),
-- BN5's sounds and collision types, and their leaving (0x080D53D4): any removal leaves a puff (BN5 neither absorbs
-- nor blinks obstacles out). Their rows (0x080D5158: each plays 0x55 × 2 ticks, BN6's by row, and pauses 0x3C,
-- 0x080D53B0; DrkSonic's row 4 its own) are the chips'. (BN5's action table has 8 rows to BN6's 10, no frozen or
-- bubbled: resting and playing are its 6 and 7, BN6's 8 and 9, the same actions.)

local collision = require("../../rules/collision")
local controller = require("@common/instruments/controller")
local effects = require("@bn6/lib/effects")
local instrument = require("@common/instruments/instrument")

local bn5_instrument = {}

bn5_instrument.sprite = asset.sprite("bn5:instrument")

bn5_instrument.kind = instrument.make_kind("bn5:instrument", {
    sprite = bn5_instrument.sprite,
    hp = 100,
    sounds = { appear = asset.sound("bn5:appear"), broken = asset.sound("bn5:hit-bomb-1") },
    body = { is = collision.own_body, hits = collision.hit_by_other_side },
    removal_puffs = true,
    -- (Its explosion and puff BN6's.)
    explosion = effects.explosion,
    puff = effects.puff,
})

-- A chip's `dimming` hook for its instrument (BN5's 0x080E5E48, BN6's `sub_80E5EA8`).
bn5_instrument.hook = controller.make("bn5:instruments/controller", bn5_instrument.kind)

return bn5_instrument
