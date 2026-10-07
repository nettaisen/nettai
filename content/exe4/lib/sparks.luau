--!strict
-- EXE4's hit sparks: what shows where a hit lands (`collision:set_hit_effect`). EXE4's spark table (0x080E2090, rows
-- 0 to 0x0C: sprite, animation, palette) is EXE5's rows 0 to 0x0C (0x080E0814) with EXE4's sprites; its spark (0x080E20E4)
-- is EXE5's code.

local sparks = {}

local HIT_SPARKS = asset.sprite("hit-sparks")

-- By element (null, fire, aqua, elec, wood, and 5): hit effects 0 to 5.
local BY_ELEMENT: { [number]: Spark } = {
    [0] = new.spark { sprite = HIT_SPARKS, anim = 0, palette = 0 },
    [1] = new.spark { sprite = HIT_SPARKS, anim = 2, palette = 0 },
    [2] = new.spark { sprite = HIT_SPARKS, anim = 3, palette = 0 },
    [3] = new.spark { sprite = HIT_SPARKS, anim = 4, palette = 0 },
    [4] = new.spark { sprite = HIT_SPARKS, anim = 5, palette = 0 },
    [5] = new.spark { sprite = asset.sprite("pink-flash"), anim = 0, palette = 0 },
}

-- The plain hit (spark 0) and the charged shot's (5).
sparks.plain = BY_ELEMENT[0]
sparks.charged = BY_ELEMENT[5]
-- The element sparks by name (hit effects 1 to 4).
sparks.fire, sparks.aqua, sparks.elec, sparks.wood = BY_ELEMENT[1], BY_ELEMENT[2], BY_ELEMENT[3], BY_ELEMENT[4]
-- A heavy hit's (6).
sparks.heavy = new.spark { sprite = HIT_SPARKS, anim = 8, palette = 0 }
-- A cannon's impact (7).
sparks.impact = new.spark { sprite = asset.sprite("impact"), anim = 0, palette = 0 }
-- A guarded hit's (8).
sparks.guard = new.spark { sprite = asset.sprite("hit"), anim = 0, palette = 0 }
-- The chip destruction hit spark (9).
sparks.chip_destruction = new.spark { sprite = asset.sprite("chip-destruction-spark"), anim = 0, palette = 0 }
-- A breaking hit (0x0A).
sparks.breaking = new.spark { sprite = HIT_SPARKS, anim = 1, palette = 0 }
-- A blast (0x0B).
sparks.blast = new.spark { sprite = asset.sprite("explosion"), anim = 0, palette = 0 }
-- The erase beams' (0x0C).
sparks.erase = new.spark { sprite = asset.sprite("pink-flash"), anim = 0, palette = 2 }

return sparks
