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

-- The erase beams' spark (hit effect 0x0C).
sparks.erase = define.spark { sprite = asset.sprite("reflected-shot"), anim = 0, palette = 2 }

-- A heavy hit's (hit effect 6): Magnum's shot, the beasts' claws and bites.
sparks.heavy = define.spark { sprite = asset.sprite("copy-mark"), anim = 8, palette = 0 }

-- The plain hit (spark 0) and the elements' (1 to 4), as named sparks:
-- what an attack that always shows one element's spark sets.
sparks.plain = BY_ELEMENT[0]
sparks.fire = BY_ELEMENT[1]
sparks.aqua = BY_ELEMENT[2]
sparks.elec = BY_ELEMENT[3]
sparks.wood = BY_ELEMENT[4]

-- A breaking hit (spark 0x0A: AuraHed's head).
sparks.breaking = define.spark { sprite = asset.sprite("copy-mark"), anim = 1, palette = 0 }

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

return sparks
