--!strict
-- Roll's heart, BN5's (attack object #0x4E, 0x080CF71C): BN6's (@exelib/roll/heart; the same code) with BN5's heart
-- sprite (10-26, animation 0) and BN5's effect table's rows 0x26 (the sparkle, 10-17) and 6 (the heal, 0c-12).

local common = require("@exelib/roll/heart")

local SPARKLE = define.effect { sprite = asset.sprite("bn5:sparkle"), anim = 0, palette = 0 }

local heart = common.make {
    id = "bn5:roll/heart",
    sprite = asset.sprite("bn5:heart"),
    anim = 0,
    heal_sound = asset.sound("bn5:heal"),
    sparkle = SPARKLE,
    healed = define.effect { sprite = asset.sprite("bn5:heal"), anim = 0, palette = 0 },
}

return { kind = heart.kind, spawn = heart.spawn, sparkle = SPARKLE }
