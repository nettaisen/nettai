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
-- Spark 0x07.
sparks.effect_0e = define.spark { sprite = asset.sprite("effect-0e"), anim = 0, palette = 0 }
-- Spark 0x08.
sparks.hit = define.spark { sprite = asset.sprite("hit"), anim = 0, palette = 0 }
-- Spark 0x09.
sparks.spark_09 = define.spark { sprite = asset.sprite("spark-09"), anim = 0, palette = 0 }
-- Spark 0x0A.
sparks.copy_mark_1 = define.spark { sprite = asset.sprite("copy-mark"), anim = 1, palette = 0 }
-- Spark 0x0B.
sparks.explosion = define.spark { sprite = asset.sprite("explosion"), anim = 0, palette = 0 }
-- Spark 0x0D.
sparks.effect_1e_1 = define.spark { sprite = asset.sprite("effect-1e"), anim = 1, palette = 0 }
-- Spark 0x0E.
sparks.spark_0e = define.spark { sprite = asset.sprite("spark-0e"), anim = 0, palette = 0 }

return sparks
