--!strict
-- Hit regions many kinds use (docs/design/content-model-v2.md §3.6): panels
-- around an anchor, { dx, dy } with dx toward the facing side, in the order
-- hits and effects go over them; or the whole field's panels that meet a
-- condition. No region is `nil`. And walking a region's panels the way the
-- game's helpers do: effects on every panel of a blast, panel types under a
-- seed.

local regions = {}

-- The anchor panel (region 0x01).
regions.single = define.region { panels = { { 0, 0 } } }

-- The anchor and the panel ahead (region 0x02: LongSwrd).
regions.long = define.region { panels = { { 0, 0 }, { 1, 0 } } }

-- The anchor and the panels above and below (region 0x04: WideSwrd).
regions.wide = define.region { panels = { { 0, 0 }, { 0, -1 }, { 0, 1 } } }

-- Three in a row from the anchor (region 0x06: FtrSword).
regions.line = define.region { panels = { { 0, 0 }, { 1, 0 }, { 2, 0 } } }

-- The anchor and its four diagonals (region 0x0B: CrosSwrd).
regions.x = define.region { panels = { { 0, 0 }, { 1, -1 }, { -1, 1 }, { 1, 1 }, { -1, -1 } } }

-- The four panels beside the anchor, and its four diagonals (regions 0x0A
-- and 0x0C: JustcOne's landing waves).
regions.beside = define.region { panels = { { 0, -1 }, { 0, 1 }, { 1, 0 }, { -1, 0 } } }
regions.diagonals = define.region { panels = { { 1, -1 }, { -1, 1 }, { 1, 1 }, { -1, -1 } } }

-- The anchor and the two panels diagonally ahead (region 0x0D: a bullet's
-- burst).
regions.fork = define.region { panels = { { 0, 0 }, { 1, -1 }, { 1, 1 } } }

-- The anchor's column and the one ahead (region 0x11: LifeSrd).
regions.wide_long = define.region {
    panels = { { 0, 0 }, { 0, -1 }, { 0, 1 }, { 1, 0 }, { 1, -1 }, { 1, 1 } },
}

-- The 3x3 around the anchor, the anchor first (region 0x0F).
regions.square = define.region {
    panels = { { 0, 0 }, { 0, -1 }, { 0, 1 }, { 1, 0 }, { -1, 0 }, { 1, -1 }, { -1, 1 }, { 1, 1 }, { -1, -1 } },
}

-- The eight around the anchor (region 0x10).
regions.ring = define.region {
    panels = { { 0, -1 }, { 0, 1 }, { 1, 0 }, { -1, 0 }, { 1, -1 }, { -1, 1 }, { 1, 1 }, { -1, -1 } },
}

-- A side's area: side 0's (region 0x81, the panels without flag 0x20) and
-- side 1's (0x82, with it).
regions.area = {
    [0] = define.region { field = { require = 0x00000000, forbid = 0x00000020 } },
    [1] = define.region { field = { require = 0x00000020, forbid = 0x00000000 } },
}

-- Every panel (region 0x80: Silence).
regions.field = define.region { field = { require = 0x00000000, forbid = 0x00000000 } }

-- Every solid panel (region 0x83: Timpani).
regions.solid = define.region { field = { require = 0x00000010, forbid = 0x00000000 } }

-- The panels of the other side's player navi: side 0's target (region
-- 0x85) and side 1's (0x84) (Discord).
regions.other_navi = {
    [0] = define.region { field = { require = 0x00200000, forbid = 0x00000000 } },
    [1] = define.region { field = { require = 0x00400000, forbid = 0x00000000 } },
}

-- `object_isValidPanel` on whole numbers (the API's panel arguments wrap
-- to a byte, which could bring a far-off panel back onto the field).
local function valid(x: number, y: number): boolean
    return x >= 1 and x <= 6 and y >= 1 and y <= 3
end

-- `sub_80CE468`: `sub_801BD3C` (`battle.region_effects`: `effect` on each
-- panel of region `r` around (x, y) at height `z`, or on each of a
-- whole-field region's from the bottom right, on the ground), skipping
-- missing panels too, and with dx stepping by `dir`
-- (`panels.side_direction`), which its callers compute from the side and
-- flip halfword.
function regions.effects_on_panels(x: number, y: number, effect: Effect, r: Region, dir: number, z: number)
    local cond = r.field
    if cond then
        for py = 3, 1, -1 do
            for px = 6, 1, -1 do
                if field.check(px, py, cond.require, cond.forbid) then
                    local cx, cy = field.center(px, py)
                    battle.effect(Vec3.new(cx, cy, 0), effect)
                end
            end
        end
        return
    end
    for _, p in r.panels or {} do
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
function regions.set_types(x: number, y: number, kind: PanelType, r: Region)
    for _, p in r.panels or {} do
        field.set_type(x + p[1], y + p[2], kind)
    end
end

return regions
