--!strict
-- EXE5's shock wave (attack object #0x16, 0x080C739C): @exelib/guard/wave with EXE5's look: its sheet (10-04) and
-- sound (0x0A6), a plain one an attack (row 4) and a marking one thrown (row 0x0A), each segment keeping its sender
-- (0x0800F1F6) and noting the panel ahead as it starts to spread. Its variant (`wave.variant { ... }`, a row of
-- 0x080C7360) is its sender's: the Guard chips' counterattack's (./init) is row 0xC.

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
    owned = true,
    notes_ahead = true,
}

return wave
