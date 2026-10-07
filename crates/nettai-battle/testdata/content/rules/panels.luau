--!strict
-- The test content's panels: made-up rules for the engine's tests. Every
-- panel type the engine has, EXE5's metal, lava and sea beside EXE6's
-- thirteen, each on the field (flag 0x10000, which the step sword looks
-- for) and solid (0x10) but the missing and the broken. The roads last
-- 0x708 ticks, lava and sea 960; lava burns for 50; sea drains fire bodies,
-- holds a body that ends a move on it for 20 ticks and submerges what can
-- dive; metal slides a body as EXE5's does. A hitbox of fire turns grass
-- normal, of aqua the volcano and lava, of wood the roads and metal.

-- A metal panel's slide, by the direction of the move (none, up, down,
-- back, forward, other): the steps tried in turn, dx toward the front.
local F, B, U, D = { dx = 1, dy = 0 }, { dx = -1, dy = 0 }, { dx = 0, dy = -1 }, { dx = 0, dy = 1 }

local function road(dx: number, dy: number)
    return { flags = 0x00010210, expires = 0x708, cleared_by = "wood", road_slide = { dx = dx, dy = dy, panels = 1 } }
end

-- Whether each panel is one of a row of the field's 6 by 3 (columns 1 to
-- 6 of rows `first` to `last`), `[y][x]` over the 8 by 5 grid.
local function rows(first: number, last: number): { { boolean } }
    local grid = {}
    for y = 0, 4 do
        local row = {}
        for x = 0, 7 do
            row[x + 1] = y >= first and y <= last and x >= 1 and x <= 6
        end
        grid[y + 1] = row
    end
    return grid
end

-- Steps: onto a free panel of one's own side (0x20 marks side 1's), solid
-- unless the body is floor-free.
local OCCUPIED = 0x0F880080
local own = {
    grounded = {
        { require = 0x00000010, forbid = 0x0F8800A0 },
        { require = 0x00000030, forbid = OCCUPIED },
    },
    floor_free = {
        { require = 0x00000000, forbid = 0x0F8800A0 },
        { require = 0x00000020, forbid = OCCUPIED },
    },
}

return {
    types = {
        missing = { flags = 0x00010000 },
        broken = { flags = 0x00010000 },
        normal = { flags = 0x00010010 },
        cracked = { flags = 0x00010050 },
        poison = { flags = 0x00010110 },
        holy = { flags = 0x00012010 },
        grass = { flags = 0x00010410, cleared_by = "fire", doubles = "fire" },
        ice = { flags = 0x00010810 },
        volcano = { flags = 0x00011010, cleared_by = "aqua" },
        road_up = road(0, -1),
        road_down = road(0, 1),
        road_left = road(-1, 0),
        road_right = road(1, 0),
        metal = {
            flags = 0x00010210,
            cleared_by = "wood",
            slide = { {}, { F, U, B, D }, { B, D, F, U }, { U, B, D, F }, { D, F, U, B }, { D, F, U, B } },
        },
        lava = { flags = 0x00011010, expires = 960, burn = { damage = 50, spared_by = 0x88000206 }, cleared_by = "aqua" },
        sea = { flags = 0x00030010, expires = 960, drains = "fire", holds = 20, submerges = true },
    },
    -- Its panel types number as the engine orders them.
    numbers = {
        "missing",
        "broken",
        "normal",
        "cracked",
        "poison",
        "holy",
        "grass",
        "ice",
        "volcano",
        "road_up",
        "road_down",
        "road_left",
        "road_right",
        "metal",
        "lava",
        "sea",
    },
    start_visible = rows(1, 3),
    front_edges = rows(3, 3),
    step = own,
    dash_step = own,
    any_side_step = {
        grounded = { { require = 0x00000010, forbid = OCCUPIED }, { require = 0x00000010, forbid = OCCUPIED } },
        floor_free = { { require = 0x00000000, forbid = OCCUPIED }, { require = 0x00000000, forbid = OCCUPIED } },
    },
    -- A broken panel mends after 0x258 ticks, 0x1E0 in battle mode 1.
    mend = { normal = 0x258, battle_mode_1 = 0x1E0 },
    -- A reservation marks its holder, whose destroy releases it.
    reservations = "marked",
    -- The flags' bits a type owns, which a crack, a break or poison clears.
    type_mask = 0x00003F5F,
    -- Poison's drain and grass's heal hold while paused, and at 9 HP or less
    -- grass heals on the 180-tick count.
    standing = { stops_while_paused = true, slow_heal_at = 9 },
}
