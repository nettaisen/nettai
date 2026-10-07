--!strict
-- Roll's heart, EXE4's (attack object #0x4E, 0x080D728C): EXE6's and EXE5's (@exelib/roll/heart; the same code) but
-- that it heals its side's navi (0x080D7354) and keeps no user, with EXE4's heart sprite (10-26, animation 0) and its
-- effect table's rows 0x2E (the sparkle, 10-17) and 7 (the heal, 0c-12), and its heal sound (0x9A).

local common = require("@exelib/roll/heart")

local SPARKLE = new.effect { sprite = asset.sprite("sparkle"), anim = 0, palette = 0 }

local heart = common.make {
    id = "roll/heart",
    sprite = asset.sprite("heart"),
    anim = 0,
    heal_sound = asset.sound("heal"),
    sparkle = SPARKLE,
    healed = new.effect { sprite = asset.sprite("heal"), anim = 0, palette = 0 },
    heals_side_navi = true,
}

return { kind = heart.kind, spawn = heart.spawn, sparkle = SPARKLE }
