--!strict
-- Panel searches the chips' objects share.

local panels = {}

-- `object_getAllianceDirection`: +1 for side 0, -1 for side 1 (the side's
-- forward, whatever the object faces).
function panels.side_direction(alliance: number): number
    return -(alliance * 2 - 1)
end

-- `GetRandomRelativePanelFiltered`: of region `region`'s panels around
-- (x, y), dx toward `dir`, one with every `require` bit and no `forbid`
-- bit, at random (one draw of the RNG, only when there is one); nil if
-- none.
function panels.random_in_region(
    x: number,
    y: number,
    require: number,
    forbid: number,
    region: number,
    dir: number
): (number?, number?)
    local shape = data.regions[region]
    if shape == nil then
        error(string.format("GetRandomRelativePanelFiltered: region %d is not in the content", region))
    end
    local found: { number } = {}
    for _, p in shape do
        local px, py = x + p[1] * dir, y + p[2]
        if field.check(px, py, require, forbid) then
            -- The game keeps them as bytes: x in the low 3 bits, y above.
            table.insert(found, bit32.bor(bit32.band(px, 0xFF), bit32.lshift(bit32.band(py, 0xFF), 4)))
        end
    end
    if #found == 0 then
        return nil, nil
    end
    local pick = found[battle.rng_positive() % #found + 1]
    return bit32.band(pick, 7), bit32.band(bit32.rshift(pick, 4), 0xF)
end

-- `off_8109784`: an enemy of each side stands there (the other side's
-- body bit).
local ENEMY_BODY = { [0] = 0x0400_0000, 0x0800_0000 }

-- `sub_8109708`: a random panel of region `region` around (x, y), forward
-- for `me`'s side, that an enemy of `me` stands on; nil if none.
function panels.random_enemy_panel(me: Object, x: number, y: number, region: number): (number?, number?)
    return panels.random_in_region(x, y, ENEMY_BODY[me.alliance], 0, region, panels.side_direction(me.alliance))
end

return panels
