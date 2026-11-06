--!strict
-- The test content's panels: made-up rules for the engine's tests. Its
-- own sixteen types, EXE5's magnet, lava and sea beside EXE6's thirteen,
-- each on the field (flag 0x10000, which the step sword looks
-- for) and solid (0x10) but the missing and the broken. The roads last
-- 0x708 ticks, lava and sea 960; lava burns for 50; sea drains fire bodies,
-- holds a body that ends a move on it for 20 ticks and submerges what can
-- dive; magnet slides a body as EXE5's does. A hitbox of fire turns grass
-- normal, of aqua the volcano and lava, of wood the roads and magnet.
-- Poison drains a body standing on it, and grass heals a wood body, at 9
-- HP or less on the 180-tick count.

local eruption = require("../objects/eruption")
local panel_types = require("@exelib/panel_types")
local statuses = require("./status")
local sprites = require("../sprites")

-- A magnet panel's slide, by the direction of the move (none, up, down,
-- back, forward, other): the steps tried in turn, dx toward the front.
local F, B, U, D = { dx = 1, dy = 0 }, { dx = -1, dy = 0 }, { dx = 0, dy = -1 }, { dx = 0, dy = 1 }

local function road(dx: number, dy: number)
    return {
        flags = { solid = true, road = true, on_field = true },
        expires = 0x708,
        cleared_by = "wood",
        carries = { dx = dx, dy = dy, panels = 1 },
        rest = panel_types.road,
        slide = panel_types.carries,
    }
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

-- Steps: onto a free panel of one's own side, solid unless the body is
-- floor-free.
local own = {
    grounded = { solid = true, own_area = true, occupied = false },
    floor_free = { own_area = true, occupied = false },
}

return {
    -- The flags the types set besides the engine's: EXE6's, and EXE5's sea.
    type_flags = {
        poison = 0x100,
        road = 0x200,
        grass = 0x400,
        ice = 0x800,
        volcano = 0x1000,
        holy = 0x2000,
        broken = 0x4000,
        missing = 0x8000,
        on_field = 0x10000,
        sea = 0x20000,
    },
    types = {
        missing = { flags = { on_field = true } },
        broken = { flags = { on_field = true } },
        normal = { flags = { solid = true, on_field = true } },
        cracked = { flags = { solid = true, cracked = true, on_field = true } },
        poison = { flags = { solid = true, poison = true, on_field = true }, stand = panel_types.poison },
        holy = { flags = { solid = true, holy = true, on_field = true }, damage_shift = 1 },
        grass = {
            flags = { solid = true, grass = true, on_field = true },
            cleared_by = "fire",
            doubles = "fire",
            stand = panel_types.grass(9),
        },
        ice = {
            flags = { solid = true, ice = true, on_field = true },
            move_end = panel_types.ice_slides,
            slide = panel_types.ice,
            hit = panel_types.freezes(statuses.freeze_150),
        },
        volcano = {
            flags = { solid = true, volcano = true, on_field = true },
            cleared_by = "aqua",
            tick = eruption.volcano,
        },
        road_up = road(0, -1),
        road_down = road(0, 1),
        road_left = road(-1, 0),
        road_right = road(1, 0),
        magnet = {
            flags = { solid = true, road = true, on_field = true },
            cleared_by = "wood",
            carries_by_move = { {}, { F, U, B, D }, { B, D, F, U }, { U, B, D, F }, { D, F, U, B }, { D, F, U, B } },
            move_end = panel_types.magnet,
            slide = panel_types.carries,
        },
        lava = {
            flags = { solid = true, volcano = true, on_field = true },
            expires = 960,
            cleared_by = "aqua",
            burn = panel_types.burn {
                damage = 50,
                spared_by = { invisible = true, submerged = true, flashing = true, untouchable = true, bubbled = true },
                spark = new.spark { sprite = sprites["test-spark"], anim = 0, palette = 0, steps = 4 },
            },
        },
        sea = {
            flags = { solid = true, on_field = true, sea = true },
            expires = 960,
            submerges = true,
            stand = panel_types.drains("fire"),
            move_end = panel_types.holds(20, new.effect { sprite = sprites["test-burst"], anim = 0, palette = 0, steps = 9 }),
            slide = panel_types.stops_a_slide,
        },
    },
    -- Its panel types by its numbers.
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
        "magnet",
        "lava",
        "sea",
    },
    -- The types the engine's own code needs.
    roles = { missing = "missing", broken = "broken", cracked = "cracked", normal = "normal" },
    start_visible = rows(1, 3),
    front_edges = rows(3, 3),
    step = own,
    dash_step = own,
    any_side_step = {
        grounded = { solid = true, occupied = false },
        floor_free = { occupied = false },
    },
    -- A broken panel mends after 0x258 ticks, 0x1E0 in battle mode 1.
    mend = { normal = 0x258, battle_mode_1 = 0x1E0 },
    -- A reservation marks its holder, whose destroy releases it.
    reservations = "marked",
    -- The flags' bits a type owns, which a crack, a break or poison clears.
    type_mask = {
        solid = true,
        cracked = true,
        poison = true,
        road = true,
        grass = true,
        ice = true,
        volcano = true,
        holy = true,
    },
    -- The field's cycle, which the volcanos erupt by.
    cycle = 0x8C,
}
