--!strict
-- SunMoon's meteor (the original's attack object #0xB5, `sub_80DBEE6`: @exelib/sun_meteor, which EXE4's RedSun's
-- shares) in EXE6's look: its sounds (0xC4 falling, 0x6F bursting), the burst (effect 5), the sparks by element. Its
-- variant (`byte_80DBEE0`'s record 0) is SunMoon's one: the fire meteor. See docs/engine/chips.md §3.6.32.

local collision = require("../../rules/collision")
local common = require("@exelib/sun_meteor")
local effects = require("../../lib/effects")
local sparks = require("../../lib/sparks")

local made = common.make {
    id = "sunmoon/meteor",
    fall_sound = asset.sound("justice-one"),
    burst_sound = asset.sound("hit-bomb-1"),
    burst = effects.burst,
    spark_of = sparks.of_element,
}

local meteor = { kind = made.kind, spawn = made.spawn }

-- The variant (`byte_80DBEE0`'s record 0, six bytes): the sprite, the animation, what it is and hits, whether the
-- landing shakes the camera (drawn only), and what it does to a panel holding something.
meteor.fire = common.variant {
    sprite = asset.sprite("meteor"),
    anim = 0,
    collision = { is = collision.thrown, hits = collision.hits_navis },
    shake = true,
    panel = "none",
}

return meteor
