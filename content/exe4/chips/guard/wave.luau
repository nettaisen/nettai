--!strict
-- EXE4's shock wave (attack object #0x16, 0x080CFBB0): @exelib/guard/wave with EXE4's look: its sheet (10-04) and
-- sound (0x08B), a plain one an attack (row 4) and a marking one thrown (row 0x0A: no row of EXE4's marks, 0x080CFB8C),
-- a segment keeping only its sender's side (0x080CFD2A) and leaving its phase byte alone as it spreads. Its variant
-- (`wave.variant { ... }`, a row of 0x080CFB8C) is its sender's: the Guard chips' counterattack's (./init) is row 6.

local collision = require("../../rules/collision")
local common = require("@exelib/guard/wave")

local wave = {}

wave.variant = common.variant

wave.spawn = common.make {
    id = "guard/wave",
    sprite = asset.sprite("shock-wave"),
    sound = asset.sound("shock-wave"),
    attack = collision.attack,
    thrown = collision.thrown,
    hits = collision.hits_navis,
    owned = false,
    notes_ahead = false,
}

return wave
