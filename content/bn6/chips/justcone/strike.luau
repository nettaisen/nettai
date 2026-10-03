--!strict
-- JustcOne's fist (the original's attack object #0xAE, `sub_80DB304`) and
-- the chip's effect (instant chip effect 19, `sub_80DB4B4`): a Break fist
-- falls from 256 pixels up and behind onto the other side's middle panel
-- (column 5 facing right, 2 facing left), 16 pixels a tick for 16 ticks.
-- It hits what it falls through. On a panel that can take it, it lands: a
-- burst, the panels around it crack, two wide hits of 100 (beside it, then
-- diagonally) that land while dimmed too, and nine puffs of dust. Then it
-- stays, blinking, for 30 ticks (unless its panel is gone). The behavior
-- is BN5's too (@common/justcone/strike), made here with BN6's look.

local collision = require("../../rules/collision")
local common = require("@common/justcone/strike")
local sparks = require("../../lib/sparks")

return common.make {
    id = "bn6:justcone/strike",
    sprite = asset.sprite("bn6:justice-one"),
    sounds = { fall = asset.sound("bn6:justice-one"), land = asset.sound("bn6:hit-bomb-1") },
    -- Its own collision (`dword_80DB558`): a thrown breaking attack that
    -- hits navis, with the breaking spark.
    is = collision.thrown_break,
    hits = collision.hits_navis,
    spark = sparks.breaking,
    wave_is = collision.attack,
    -- Break (secondary element 0x10); the landing's hits are of none.
    element = 0x10,
    waves_take_element = false,
}
