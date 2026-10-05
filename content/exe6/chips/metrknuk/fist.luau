--!strict
-- A MetrKnuk fist (the original's attack object #0xB4, `sub_80DBCEC`;
-- docs/engine/dimming-chip-effects.md §18): the shared fist
-- (@exelib/metrknuk/fist), EXE6's kind. It falls from high up behind its
-- panel onto it, hits it with a burst, cracks it and shakes the camera,
-- then bounces back up and goes.

local collision = require("../../rules/collision")
local common = require("@exelib/metrknuk/fist")
local effects = require("../../lib/effects")
local sparks = require("../../lib/sparks")

-- Its hit (`sub_80DBEBA`, 0x15050001): a breaking thrown attack hitting
-- navis on its panel, with the null spark.
return common.make {
    id = "metrknuk/fist",
    sprite = asset.sprite("justice-one"),
    sound = asset.sound("justice-one"),
    burst = effects.burst,
    self_type = collision.thrown_break,
    target = collision.hits_navis,
    spark = sparks.of_element(0),
    ends_with_battle = false,
}
