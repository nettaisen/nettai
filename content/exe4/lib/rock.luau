--!strict
-- What EXE4's rocks break into and leave (the stages' boulder, objects/boulder): the rock debris (its effect object
-- 0x080E6150, EXE6's #0x38 `sub_80E46D8`, the same code: @exelib/rock/debris) on EXE4's sheet 10-09, and the effects
-- from EXE4's table (0x080E1698: sprite, animation and palette by row).

local common_debris = require("@exelib/rock/debris")

local rock = {}

local SMALL_PUFF = asset.sprite("small-puff")
-- Effects 1 and 2 (rows 1, 2: 14-02's animations 0 and 1): a splash, dust; effect 21 (row 21: 14-0D's animation 2)
-- a puff.
rock.effects = {
    splash = new.effect { sprite = SMALL_PUFF, anim = 0, palette = 0 },
    dust = new.effect { sprite = SMALL_PUFF, anim = 1, palette = 0 },
    puff = new.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 },
}

-- The debris (0x080E6170: sheet 10-09, landing in a splash, 0x080E6208).
rock.debris = common_debris.make_kind("rock/debris", asset.sprite("rock-debris"), rock.effects.splash)

return rock
