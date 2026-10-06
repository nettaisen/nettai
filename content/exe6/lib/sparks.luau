--!strict
-- Hit sparks: what shows where a hit lands (`collision:set_hit_effect`).

local sparks = {}

-- By element (null, fire, aqua, elec, wood, and 5), the spark
-- `sub_801A146` gives an attack of that element (hit effects 0 to 5).
local BY_ELEMENT: { [number]: Spark } = {
    [0] = new.spark { sprite = asset.sprite("hit-sparks"), anim = 0, palette = 0 },
    [1] = new.spark { sprite = asset.sprite("hit-sparks"), anim = 2, palette = 0 },
    [2] = new.spark { sprite = asset.sprite("hit-sparks"), anim = 3, palette = 0 },
    [3] = new.spark { sprite = asset.sprite("hit-sparks"), anim = 4, palette = 0 },
    [4] = new.spark { sprite = asset.sprite("hit-sparks"), anim = 5, palette = 0 },
    [5] = new.spark { sprite = asset.sprite("pink-flash"), anim = 0, palette = 0 },
}

-- The element sparks by name (hit effects 1 to 4), for hits that name
-- theirs rather than take their element's.
sparks.fire, sparks.aqua, sparks.elec, sparks.wood = BY_ELEMENT[1], BY_ELEMENT[2], BY_ELEMENT[3], BY_ELEMENT[4]

-- The plain hit (spark 0) and the charged shot's (5), as named sparks.
sparks.plain = BY_ELEMENT[0]
sparks.charged = BY_ELEMENT[5]

-- The element sparks by name (hit effects 1 to 3: a flame's, a bubble
-- star's, a thunder ball's).
sparks.fire = BY_ELEMENT[1]
sparks.aqua = BY_ELEMENT[2]
sparks.elec = BY_ELEMENT[3]

-- A cannon's impact (spark 7: the cannons, AirShot).
sparks.impact = new.spark { sprite = asset.sprite("impact"), anim = 0, palette = 0 }

-- A breaking hit (spark 0x0A: the break family's shots, a thrown obstacle,
-- JustcOne's fist, a rock; GroundMan's drill and rocks, DustMan's junk,
-- BigHook's flames).
sparks.breaking = new.spark { sprite = asset.sprite("hit-sparks"), anim = 1, palette = 0 }

-- The chip destruction hit spark (spark 9: the chip destruction shot's).
sparks.chip_destruction = new.spark { sprite = asset.sprite("chip-destruction-spark"), anim = 0, palette = 0 }

-- A blast (spark 0x0B: GigaCan's shot).
sparks.blast = new.spark { sprite = asset.sprite("explosion"), anim = 0, palette = 0 }

-- A bat's impact (spark 0x0D: BatCan's shot).
sparks.bat = new.spark { sprite = asset.sprite("hit-marker"), anim = 1, palette = 0 }

-- The element sparks by name (hit effects 1 to 3), for hits that name
-- theirs rather than take their element's.
sparks.fire, sparks.aqua, sparks.elec = BY_ELEMENT[1], BY_ELEMENT[2], BY_ELEMENT[3]

-- The wood spark (hit effect 4): the boomerang's, the lances'.
sparks.wood = BY_ELEMENT[4]

-- The erase beams' spark (hit effect 0x0C).
sparks.erase = new.spark { sprite = asset.sprite("pink-flash"), anim = 0, palette = 2 }

-- A heavy hit's (hit effect 6): Magnum's shot, the beasts' claws and bites,
-- the flying shot's arrows, VDoll's curse, JudgeMan's books.
sparks.heavy = new.spark { sprite = asset.sprite("hit-sparks"), anim = 8, palette = 0 }

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
sparks.guard = new.spark { sprite = asset.sprite("hit"), anim = 0, palette = 0 }
-- Spark 0x0E.
sparks.uninstall = new.spark { sprite = asset.sprite("spark-0e"), anim = 0, palette = 0 }

return sparks
