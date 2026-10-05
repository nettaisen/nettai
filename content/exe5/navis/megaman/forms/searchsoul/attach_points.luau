--!strict
-- The attach points of MegaMan's battle sprite in SearchSoul (00-03), in pixels, x toward the facing side: the
-- sprite's row of the attach points' category 0 table (0x080151CC's row 3: 30 points, 0x3C bytes; the base
-- form's row is navis/megaman/attach_points).

local points: { { number } } = {
    { 0, 0 },
    { 0, 0 },
    { 0, 0 },
    { 8, 47 },
    { 8, 50 },
    { 8, 50 },
    { 16, 0 },
    { 16, 24 },
    { 16, 24 },
    { 24, 24 },
    { 18, 24 },
    { 18, 24 },
    { 18, 24 },
    { 24, 24 },
    { 24, 24 },
    { 24, 24 },
    { 40, 26 },
    { 0, 18 },
    { 24, 28 },
    { 40, 26 },
    { -1, 26 },
    { 0, 16 },
    { 24, 26 },
    { 24, 26 },
    { 24, 23 },
    { 24, 26 },
    { 0, 18 },
    { 0, 18 },
    { 24, 25 },
    { 40, 20 },
}

return points
