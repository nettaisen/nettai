--!strict
-- Hit sparks: what shows where a hit lands (`collision:set_hit_effect`).

local sparks = {}

-- By element (null, fire, aqua, elec, wood, and 5), the spark
-- `sub_801A146` gives an attack of that element (hit effects 0 to 5).
local BY_ELEMENT: { [number]: Spark } = {
    [0] = define.spark { sprite = asset.sprite("copy-mark"), anim = 0, palette = 0 },
    [1] = define.spark { sprite = asset.sprite("copy-mark"), anim = 2, palette = 0 },
    [2] = define.spark { sprite = asset.sprite("copy-mark"), anim = 3, palette = 0 },
    [3] = define.spark { sprite = asset.sprite("copy-mark"), anim = 4, palette = 0 },
    [4] = define.spark { sprite = asset.sprite("copy-mark"), anim = 5, palette = 0 },
    [5] = define.spark { sprite = asset.sprite("reflected-shot"), anim = 0, palette = 0 },
}

-- The element sparks by name (hit effects 1 to 4), for hits that name
-- theirs rather than take their element's.
sparks.fire, sparks.aqua, sparks.elec, sparks.wood = BY_ELEMENT[1], BY_ELEMENT[2], BY_ELEMENT[3], BY_ELEMENT[4]

-- The plain hit (spark 0) and the charged shot's (5), as named sparks.
sparks.plain = BY_ELEMENT[0]
sparks.charged = BY_ELEMENT[5]

-- A cannon's impact (spark 7: the cannons, AirShot).
sparks.impact = define.spark { sprite = asset.sprite("impact"), anim = 0, palette = 0 }

-- A breaking hit (spark 0x0A: the break family's shots, a thrown obstacle,
-- JustcOne's fist, a rock; GroundMan's drill and rocks, DustMan's junk,
-- BigHook's flames).
sparks.breaking = define.spark { sprite = asset.sprite("copy-mark"), anim = 1, palette = 0 }

-- A shot's impact (spark 9: a projectile variant nothing names yet).
sparks.shot = define.spark { sprite = asset.sprite("shot-impact"), anim = 0, palette = 0 }

-- A blast (spark 0x0B: GigaCan's shot).
sparks.blast = define.spark { sprite = asset.sprite("explosion"), anim = 0, palette = 0 }

-- A bat's impact (spark 0x0D: BatCan's shot).
sparks.bat = define.spark { sprite = asset.sprite("bat-impact"), anim = 1, palette = 0 }

-- The element sparks by name (hit effects 1 to 3), for hits that name
-- theirs rather than take their element's.
sparks.fire, sparks.aqua, sparks.elec = BY_ELEMENT[1], BY_ELEMENT[2], BY_ELEMENT[3]

-- The wood spark (hit effect 4): the boomerang's, the lances'.
sparks.wood = BY_ELEMENT[4]

-- The erase beams' spark (hit effect 0x0C).
sparks.erase = define.spark { sprite = asset.sprite("reflected-shot"), anim = 0, palette = 2 }

-- A heavy hit's (hit effect 6): Magnum's shot, the beasts' claws and bites,
-- the flying shot's arrows, VDoll's curse, JudgeMan's books.
sparks.heavy = define.spark { sprite = asset.sprite("copy-mark"), anim = 8, palette = 0 }

-- `sub_801A146`: the spark of an attack of `element` (its primary
-- element, the low nibble).
function sparks.of_element(element: number): Spark
    local e = bit32.band(element, 0xF)
    local s = BY_ELEMENT[e]
    if not s then
        error(string.format("sub_801A146: element %d has no hit spark", e))
    end
    return s
end

-- The rest of the original's sparks (gen-content).
-- Spark 0x08.
sparks.hit = define.spark { sprite = asset.sprite("hit"), anim = 0, palette = 0 }
-- Spark 0x0E.
sparks.spark_0e = define.spark { sprite = asset.sprite("spark-0e"), anim = 0, palette = 0 }

return sparks
