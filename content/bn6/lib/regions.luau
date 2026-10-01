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

-- The halves of that X: the diagonal rising forward (region 0x12) and the
-- one falling forward (0x13), TwinLdrs's Colonel striking both.
regions.rising = define.region { panels = { { 0, 0 }, { 1, -1 }, { -1, 1 } } }
regions.falling = define.region { panels = { { 0, 0 }, { -1, -1 }, { 1, 1 } } }

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

-- The anchor and its four neighbours (region 0x09: H-Burst's even bursts).
regions.plus = define.region { panels = { { 0, 0 }, { 0, -1 }, { 0, 1 }, { 1, 0 }, { -1, 0 } } }

-- The anchor and the panel above it (region 0x18: a big rolling log).
regions.tall = define.region { panels = { { 0, 0 }, { 0, -1 } } }

-- The anchor and the column ahead of it (region 0x15: ElcPuls's pulse).
regions.fan = define.region { panels = { { 0, 0 }, { 1, -1 }, { 1, 1 }, { 1, 0 } } }

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

-- The rest of the original's regions (gen-content).
-- Region 0x03.
regions.region_03 = define.region { panels = { { 1, 0 } } }
-- Region 0x05.
regions.region_05 = define.region { panels = { { 0, -1 }, { 0, 1 } } }
-- Region 0x07.
regions.region_07 = define.region { panels = { { 0, 0 }, { 1, 0 }, { 2, 0 }, { 3, 0 } } }
-- Region 0x08.
regions.region_08 = define.region { panels = { { 0, 0 }, { 1, 0 }, { 2, 0 }, { 3, 0 }, { 4, 0 }, { 5, 0 } } }
-- Region 0x0E.
regions.region_0e = define.region { panels = { { 1, -1 }, { 1, 1 } } }
-- Region 0x14.
regions.region_14 = define.region { panels = { { 0, 0 }, { 0, -1 }, { 0, 1 }, { 1, 0 } } }
-- Region 0x16.
regions.region_16 = define.region { panels = { { 0, 0 }, { 1, -1 } } }
-- Region 0x17.
regions.region_17 = define.region { panels = { { 0, 0 }, { 1, 1 } } }
-- Region 0x19.
regions.region_19 = define.region { panels = { { 0, 0 }, { 0, 1 } } }
-- Region 0x1A.
regions.region_1a = define.region { panels = { { 0, 0 }, { 1, -1 }, { 1, 0 }, { 1, 1 }, { 2, -1 }, { 2, 0 }, { 2, 1 } } }
-- Region 0x1B.
regions.region_1b = define.region { panels = { { 0, 0 }, { 1, -1 }, { 1, 0 }, { 1, 1 }, { 2, -1 }, { 2, 0 }, { 2, 1 }, { 3, -1 }, { 3, 0 }, { 3, 1 } } }
-- Region 0x1C.
regions.region_1c = define.region { panels = { { 0, 0 }, { 1, -1 }, { 1, 0 }, { 1, 1 }, { 2, -1 }, { 2, 0 }, { 2, 1 }, { 3, -1 }, { 3, 0 }, { 3, 1 }, { 4, -1 }, { 4, 0 }, { 4, 1 } } }
-- Region 0x1D.
regions.region_1d = define.region { panels = { { 0, 0 }, { 1, 0 }, { -1, 0 } } }
-- Region 0x1E.
regions.region_1e = define.region { panels = { { 1, 0 }, { -1, 0 } } }
-- Region 0x1F.
regions.region_1f = define.region { panels = { { 0, 0 }, { -1, -1 }, { -1, 1 } } }
-- Region 0x20.
regions.region_20 = define.region { panels = { { 0, 0 }, { 1, 0 }, { 2, 0 }, { 3, 0 }, { 4, 0 } } }
-- Region 0x21.
regions.region_21 = define.region { panels = { { 0, 0 }, { 0, -1 }, { 0, 1 }, { 1, 0 }, { 1, -1 }, { 1, 1 }, { 2, 0 }, { 2, -1 }, { 2, 1 }, { 3, 0 }, { 3, -1 }, { 3, 1 } } }
-- Region 0x22.
regions.region_22 = define.region { panels = { { -1, -1 }, { 0, -1 }, { 1, -1 }, { 0, 0 }, { -1, 1 }, { 0, 1 }, { 1, 1 } } }
-- Region 0x23.
regions.region_23 = define.region { panels = { { 0, -1 }, { 1, -1 }, { 2, -1 }, { 3, -1 }, { 4, -1 }, { 5, -1 }, { 0, 0 }, { 1, 0 }, { 2, 0 }, { 3, 0 }, { 4, 0 }, { 5, 0 }, { 0, 1 }, { 1, 1 }, { 2, 1 }, { 3, 1 }, { 4, 1 }, { 5, 1 } } }
-- Region 0x24.
regions.region_24 = define.region { panels = { { 0, -1 }, { 1, -1 }, { 2, -1 }, { -1, 0 }, { 0, 0 }, { 1, 0 }, { -2, 1 }, { -1, 1 }, { 0, 1 } } }
-- Region 0x25.
regions.region_25 = define.region { panels = { { -2, -1 }, { -1, -1 }, { 0, -1 }, { -1, 0 }, { 0, 0 }, { 1, 0 }, { 0, 1 }, { 1, 1 }, { 2, 1 } } }
-- Region 0x26.
regions.region_26 = define.region { panels = { { -2, -2 }, { -1, -2 }, { 0, -2 }, { 1, -2 }, { 2, -2 }, { -2, -1 }, { -1, -1 }, { 0, -1 }, { 1, -1 }, { 2, -1 }, { -2, 0 }, { -1, 0 }, { 0, 0 }, { 1, 0 }, { 2, 0 }, { -2, 1 }, { -1, 1 }, { 0, 1 }, { 1, 1 }, { 2, 1 }, { -2, 2 }, { -1, 2 }, { 0, 2 }, { 1, 2 }, { 2, 2 } } }
-- Region 0x27.
regions.region_27 = define.region { panels = { { 0, -1 }, { 1, -1 }, { 2, -1 }, { 3, -1 }, { 4, -1 }, { 5, -1 }, { 5, 0 }, { 0, 1 }, { 1, 1 }, { 2, 1 }, { 3, 1 }, { 4, 1 }, { 5, 1 } } }
-- Region 0x28.
regions.region_28 = define.region { panels = { { 0, 0 }, { 1, 0 }, { 2, 0 }, { 2, -1 }, { 2, 1 } } }
-- Region 0x29.
regions.region_29 = define.region { panels = { { 0, -1 }, { 1, -1 }, { 2, -1 }, { 1, 0 }, { 0, 1 }, { 1, 1 }, { 2, 1 } } }
-- Region 0x2A.
regions.region_2a = define.region { panels = { { 0, -1 }, { 0, 1 }, { 0, 0 }, { 1, 0 }, { 2, 0 }, { 3, 0 }, { 4, 0 }, { 5, 0 } } }
-- Region 0x2B.
regions.region_2b = define.region { panels = { { 0, 0 }, { -1, 0 }, { -2, 0 } } }
-- Region 0x2C.
regions.region_2c = define.region { panels = { { 5, -2 }, { 4, -2 }, { 3, -2 }, { 2, -2 }, { 1, -2 }, { 0, -2 }, { -1, -2 }, { -2, -2 }, { -3, -2 }, { -4, -2 }, { -5, -2 }, { 5, 2 }, { 4, 2 }, { 3, 2 }, { 2, 2 }, { 1, 2 }, { 0, 2 }, { -1, 2 }, { -2, 2 }, { -3, 2 }, { -4, 2 }, { -5, 2 }, { 5, -1 }, { 4, -1 }, { 3, -1 }, { 2, -1 }, { -2, -1 }, { -3, -1 }, { -4, -1 }, { -5, -1 }, { 5, 1 }, { 4, 1 }, { 3, 1 }, { 2, 1 }, { -2, 1 }, { -3, 1 }, { -4, 1 }, { -5, 1 }, { 5, 0 }, { 4, 0 }, { 3, 0 }, { 2, 0 }, { -2, 0 }, { -3, 0 }, { -4, 0 }, { -5, 0 } } }
-- Region 0x2D.
regions.region_2d = define.region { panels = { { 0, 0 }, { 0, -2 }, { 0, -1 }, { 0, 1 }, { 0, 2 } } }
-- Region 0x2E.
regions.region_2e = define.region { panels = { { 0, -2 }, { 0, -1 }, { 0, 1 }, { 0, 2 } } }
-- Region 0x86: the field's panels with every `require` bit and no `forbid` bit.
regions.field_no_navi = define.region { field = { require = 0x00000000, forbid = 0x00600000 } }
-- Region 0x87: the field's panels with every `require` bit and no `forbid` bit.
regions.field_solid_left = define.region { field = { require = 0x00000010, forbid = 0x00000020 } }
-- Region 0x88: the field's panels with every `require` bit and no `forbid` bit.
regions.field_solid_right = define.region { field = { require = 0x00000030, forbid = 0x00000000 } }

return regions
