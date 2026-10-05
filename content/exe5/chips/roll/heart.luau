--!strict
-- Roll's heart, EXE5's (attack object #0x4E, 0x080CF71C): EXE6's (@exelib/roll/heart; the same code) with EXE5's heart
-- sprite (10-26, animation 0) and EXE5's effect table's rows 0x26 (the sparkle, 10-17) and 6 (the heal, 0c-12).

local common = require("@exelib/roll/heart")

local SPARKLE = define.effect { sprite = asset.sprite("sparkle"), anim = 0, palette = 0 }

local heart = common.make {
    id = "roll/heart",
    sprite = asset.sprite("heart"),
    anim = 0,
    heal_sound = asset.sound("heal"),
    sparkle = SPARKLE,
    healed = define.effect { sprite = asset.sprite("heal"), anim = 0, palette = 0 },
}

return { kind = heart.kind, spawn = heart.spawn, sparkle = SPARKLE }
