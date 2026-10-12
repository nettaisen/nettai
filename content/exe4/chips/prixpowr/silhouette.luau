--!strict
-- LaserMan's silhouette, as PrixPowr shows it (sprite 10-3E, through the afterimage, effect object #0x28,
-- 0x080E5020): where its VideoMan projects him, and where its LaserMan was as he goes. A copy that blinks for its
-- ticks, with no shadow or color shader, facing its maker's side's way.

local SPRITE = asset.sprite("laserman-silhouette")

-- The silhouette on (x, y)'s center for `ticks`, for `maker`.
local function show(maker: Object, x: number, y: number, ticks: number)
    local cx, cy = field.center(x, y)
    battle.afterimage(maker, Vec3.new(cx, cy, 0), {
        sprite = SPRITE,
        anim = 0,
        flip = maker.alliance,
        lifetime = ticks,
        color_shader = 0,
    })
end

return { show = show }
