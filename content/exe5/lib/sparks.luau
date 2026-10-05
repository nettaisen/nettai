--!strict
-- EXE5's hit sparks: what shows where a hit lands (`collision:set_hit_effect`). EXE5's spark table (0x080E0814,
-- rows 0 to 0xD) has EXE6's rows (`off_80E0804`: sprite, animation, palette) with EXE5's sprites; EXE5's hit sparks
-- (14-05) last a frame longer than EXE6's, and a spark goes when its animation ends, so EXE5's hits show EXE5's
-- (an EXE6 module that sets a spark takes a game's: chips/thunder's look).

local sparks = {}

local HIT_SPARKS = asset.sprite("hit-sparks")

-- By element (null, fire, aqua, elec, wood, and 5): the spark the element routine (EXE6's `sub_801A146`, EXE5's
-- the same code) gives an attack of that element (hit effects 0 to 5).
local BY_ELEMENT: { [number]: Spark } = {
    [0] = define.spark { sprite = HIT_SPARKS, anim = 0, palette = 0 },
    [1] = define.spark { sprite = HIT_SPARKS, anim = 2, palette = 0 },
    [2] = define.spark { sprite = HIT_SPARKS, anim = 3, palette = 0 },
    [3] = define.spark { sprite = HIT_SPARKS, anim = 4, palette = 0 },
    [4] = define.spark { sprite = HIT_SPARKS, anim = 5, palette = 0 },
    [5] = define.spark { sprite = asset.sprite("pink-flash"), anim = 0, palette = 0 },
}

-- The plain hit (spark 0) and the charged shot's (5).
sparks.plain = BY_ELEMENT[0]
sparks.charged = BY_ELEMENT[5]
-- The element sparks by name (hit effects 1 to 4), for hits that name theirs.
sparks.fire, sparks.aqua, sparks.elec, sparks.wood = BY_ELEMENT[1], BY_ELEMENT[2], BY_ELEMENT[3], BY_ELEMENT[4]
-- A heavy hit's (6).
sparks.heavy = define.spark { sprite = HIT_SPARKS, anim = 8, palette = 0 }
-- A cannon's impact (7).
sparks.impact = define.spark { sprite = asset.sprite("impact"), anim = 0, palette = 0 }
-- A guarded hit's (8).
sparks.guard = define.spark { sprite = asset.sprite("hit"), anim = 0, palette = 0 }
-- A shot's impact (9).
sparks.shot = define.spark { sprite = asset.sprite("shot-impact"), anim = 0, palette = 0 }
-- A breaking hit (0x0A).
sparks.breaking = define.spark { sprite = HIT_SPARKS, anim = 1, palette = 0 }
-- A blast (0x0B).
sparks.blast = define.spark { sprite = asset.sprite("explosion"), anim = 0, palette = 0 }
-- The erase beams' (0x0C).
sparks.erase = define.spark { sprite = asset.sprite("pink-flash"), anim = 0, palette = 2 }
-- A bat's impact (0x0D).
sparks.bat = define.spark { sprite = asset.sprite("hit-marker"), anim = 1, palette = 0 }

-- The element routine: the spark of an attack of `element` (its primary element, the low nibble).
function sparks.of_element(element: number): Spark
    local e = bit32.band(element, 0xF)
    local s = BY_ELEMENT[e]
    if not s then
        error(string.format("the element spark routine: element %d has no hit spark", e))
    end
    return s
end

return sparks
