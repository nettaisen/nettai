--!strict
-- EXE4's instruments (attack object #0x78, 0x080DC33C) and their dimming controller (the dimming chips' variant 0x24:
-- effect object #0x4A, 0x080E7310, its effect 0x080E7354): the shared code (@exelib/instruments: EXE6's, as EXE5's)
-- with EXE4's look: the sprite (04-08); EXE4's sounds (it appears with 0xB0, leaves with 0x70) and collision types;
-- no NameID written (0x080DC35C: no identity); and its leaving (0x080DC58C): whatever removes it, it explodes as when
-- broken (EXE4 neither absorbs nor blinks obstacles out). The controller's spawn (0x080E73B8) and the instrument's
-- (0x080DC64C) take their spawner's side alone. A chip's instrument is a row of 0x080DC32C (each plays 0x55 × 2
-- ticks, its effect every third tick, its tune every 0x40), its HP (0x080DC3EC: 50 for row 0, else 100), its tune
-- (0x080DC54C), its effect (0x080DC53C) and its pause (0x080DC56C: 60), which each chip states.

local collision = require("../rules/collision")
local controller = require("@exelib/instruments/controller")
local instrument = require("@exelib/instruments/instrument")

local instruments = {}

instruments.sprite = asset.sprite("instrument")

-- Its HP but in row 0 (0x080DC3EC).
local HP = 100

instruments.kind = instrument.make_kind("instrument", {
    sprite = instruments.sprite,
    hp = HP,
    sounds = { appear = asset.sound("appear"), broken = asset.sound("hit-bomb-1") },
    body = { is = collision.own_body, hits = collision.hit_by_other_side },
    removal_explodes = true,
    -- 0x080DC5A2: effect row 0, 14-00.
    explosion = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0 },
})

-- A chip's `dimming` hook for its instrument (0x080E73B8).
instruments.hook = controller.make("instruments/controller", instruments.kind, true)

return instruments
