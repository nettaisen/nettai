--!strict
-- The attach points of MegaMan's battle sprite (00-00): the first row of the attach points' category 0 table
-- (0x080151CC, from 0x080151C0's list of the categories' tables; 0x3C bytes a sprite: BN6's have 34), in pixels, x
-- toward the facing side. Dark MegaMan (navis/dark-megaman) wears the same sprite.

local points: { { number } } = {
    { 0, 0 },
    { 0, 0 },
    { 0, 0 },
    { 8, 48 },
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
