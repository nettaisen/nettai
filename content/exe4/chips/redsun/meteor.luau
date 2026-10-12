--!strict
-- RedSun's meteor (EXE4's attack object #0x85, 0x080DDDEE: @exelib/sun_meteor, EXE6's SunMoon's code) in EXE4's look:
-- its sounds (0x146 falling, 0x70 bursting), the burst (effect row 6), the sparks by element. Its variant (0x080DDDE8's
-- row 0) is the sun's one.

local collision = require("../../rules/collision")
local common = require("@exelib/sun_meteor")
local sparks = require("../../lib/sparks")

local made = common.make {
    id = "redsun/meteor",
    fall_sound = asset.sound("justice-one"),
    burst_sound = asset.sound("hit-bomb-1"),
    burst = new.effect { sprite = asset.sprite("burst"), anim = 0, palette = 0 },
    spark_of = sparks.of_element,
}

local meteor = { kind = made.kind, spawn = made.spawn }

-- The variant (0x080DDDE8's row 0, six bytes): the meteor sheet's animation 0, a thrown attack hitting navis, the
-- landing shaking the camera, no panel touched.
meteor.fire = common.variant {
    sprite = asset.sprite("meteor"),
    anim = 0,
    collision = { is = collision.thrown, hits = collision.hits_navis },
    shake = true,
    panel = "none",
}

return meteor
