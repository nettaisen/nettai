--!strict
-- Walking a hit region's panels (data.regions, or a whole-field region
-- 0x80 + i from data.rules.field_regions) the way the game's helpers do:
-- effects on every panel of a blast, panel types under a seed.

local region = {}

-- `object_isValidPanel` on whole numbers (the API's panel arguments wrap
-- to a byte, which could bring a far-off panel back onto the field).
local function valid(x: number, y: number): boolean
    return x >= 1 and x <= 6 and y >= 1 and y <= 3
end

-- One region's panels, relative ({ dx, dy } each); none for region 0.
local function shape(r: number): { { number } }
    local panels = data.regions[r]
    if not panels then
        error(string.format("hit region %#x is not in the content", r))
    end
    return panels
end

-- The whole-field branch: every panel, bottom right first, whose flags
-- word meets the condition, gets the effect at height 0.
local function field_effects(r: number, effect: number)
    local cond = data.rules.field_regions[r]
    if not cond then
        error(string.format("whole-field region %#x is not in the content", 0x80 + r))
    end
    for y = 3, 1, -1 do
        for x = 6, 1, -1 do
            if field.check(x, y, cond.require, cond.forbid) then
                local cx, cy = field.center(x, y)
                battle.effect(Vec3.new(cx, cy, 0), effect)
            end
        end
    end
end

-- `sub_80CE468`: `sub_801BD3C` (`battle.region_effects`: effect `effect`
-- on each panel of region `r` around (x, y) at height `z`, or the
-- whole-field region's), skipping missing panels too, and with dx stepping
-- by `dir` (`panels.side_direction`), which its callers compute from the
-- side and flip halfword.
function region.effects_on_panels(x: number, y: number, effect: number, r: number, dir: number, z: number)
    r = bit32.band(r, 0xFF)
    if bit32.btest(r, 0x80) then
        return field_effects(bit32.band(r, 0x7F), effect)
    end
    if r == 0 then
        return
    end
    for _, p in shape(r) do
        local px, py = x + p[1] * dir, y + p[2]
        if valid(px, py) then
            local panel = field.panel(px, py)
            if panel and panel.kind ~= "missing" then
                local cx, cy = field.center(px, py)
                battle.effect(Vec3.new(cx, cy, z), effect)
            end
        end
    end
end

-- `sub_80CE424`: panel type `kind` on each panel of region `r` around
-- (x, y), dx not mirrored (`object_setPanelType` leaves missing panels,
-- and the field's border, alone).
function region.set_types(x: number, y: number, kind: PanelType, r: number)
    for _, p in shape(r) do
        field.set_type(x + p[1], y + p[2], kind)
    end
end

return region
