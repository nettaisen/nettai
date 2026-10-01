--!strict
-- What a Cross wears over MegaMan's body: a body overlay in its own
-- palette. The Crosses' overlays share one depth table (`off_80C42D4`):
-- drawn behind MegaMan in his animations 0x17 to 0x2B, 0x40 to 0x67 and
-- 0x7B.

return function(sprite: SpriteAsset): Parts
    return {
        body = {
            sprite = sprite,
            anims = 124,
            behind = { { 0x17, 0x2B }, { 0x40, 0x67 }, { 0x7B, 0x7B } },
            own_palette = true,
        },
    }
end
