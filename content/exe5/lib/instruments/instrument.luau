--!strict
-- EXE5's instruments (attack object #0x78) and their dimming controller (effect object #0x4A): the shared code
-- (@exelib/instruments: EXE6's) but for their look: the sprite (04-08), 100 HP (0x080D5224, EXE6's 60),
-- EXE5's sounds and collision types, and their leaving (0x080D53D4): any removal leaves a puff (EXE5 neither absorbs
-- nor blinks obstacles out). Their rows (0x080D5158: each plays 0x55 × 2 ticks, EXE6's by row, and pauses 0x3C,
-- 0x080D53B0; DrkSonic's row 4 its own) are the chips'. (EXE5's action table has 8 rows to EXE6's 10, no frozen or
-- bubbled: resting and playing are its 6 and 7, EXE6's 8 and 9, the same actions.)

local collision = require("../../rules/collision")
local controller = require("@exelib/instruments/controller")
local effects = require("../effects")
local instrument = require("@exelib/instruments/instrument")

local exe5_instrument = {}

exe5_instrument.sprite = asset.sprite("instrument")

exe5_instrument.kind = instrument.make_kind("instrument", {
    sprite = exe5_instrument.sprite,
    hp = 100,
    sounds = { appear = asset.sound("appear"), broken = asset.sound("hit-bomb-1") },
    body = { is = collision.own_body, hits = collision.hit_by_other_side },
    removal_puffs = true,
    -- (Its explosion and puff EXE6's.)
    explosion = effects.explosion,
    puff = effects.puff,
})

-- A chip's `dimming` hook for its instrument (EXE5's 0x080E5E48, EXE6's `sub_80E5EA8`).
exe5_instrument.hook = controller.make("instruments/controller", exe5_instrument.kind)

return exe5_instrument
