--!strict
-- Whom ShadowMan's attacks find (0x08013018, EXE6's `sub_8016218`, as AntiDmg's counter's variant 1 does): the nearest
-- enemy body ahead of him, column by column to the field's edge, in each column his own row first.

local aim = {}

-- An enemy's body, by side (0x08013090), and the rows looked at, by his row (0x08013098).
local ENEMY_BODY: { [number]: number } = { [0] = 0x0400_0000, [1] = 0x0800_0000 }
local ROWS: { [number]: { number } } = { [1] = { 1, 2, 3 }, [2] = { 2, 1, 3 }, [3] = { 3, 1, 2 } }

-- The panel of the nearest enemy body ahead of `me`; none: nil.
function aim.nearest_enemy(me: Object): (number?, number?)
    local rows = ROWS[me.panel_y]
    if not rows then
        error(string.format("0x08013018 reads past 0x08013098 for row %d", me.panel_y))
    end
    local body = ENEMY_BODY[me.alliance]
    local x = me.panel_x + me.facing
    while field.valid(x, 2) do
        for _, y in rows do
            if bit32.btest(field.flags(x, y), body) then
                return x, y
            end
        end
        x += me.facing
    end
    return nil, nil
end

return aim
