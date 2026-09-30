--!strict
-- The panel changer (effect object #0x1F, `sub_80E28A8`): it lists the
-- panels its kind's row picks (all, its row, the panel in front, or its
-- own; those meeting its side's condition), shows them as the new type
-- while its timer runs, then changes them, and clears its spawner's flag.
-- AntiRecv's counterattack (objects/anti-recovery) poisons the healer's
-- panel with kind 6; the panel chips' controller (dimming chip subtype 5,
-- `sub_80E2AE8`: PnlRetrn, HolyPanl, Snctuary, ComingRd, GoingRd) spawns it
-- with the chip's Param1. See docs/engine/dimming-chips.md §4.2.
--
-- Its spawner (`sub_80E2ACA`) is exported: `panel_changer.spawn`.

local panels = require("../../lib/panels")

export type State = {
    -- The kind's row (ExtraVars+0, as an index: its Param1 after init).
    kind: number,
    -- The panels listed (the object's +0x68, packed y << 4 | x), and how
    -- many (ExtraVars+4).
    list: { number },
    count: number,
}

local panel_changer = {
    state = { kind = "u8", list = "u8[18]", count = "u8" },
}

-- Panel types by the game's number.
local TYPES: { [number]: PanelType } = {
    [0x0] = "missing",
    [0x1] = "broken",
    [0x2] = "normal",
    [0x3] = "cracked",
    [0x4] = "poison",
    [0x5] = "holy",
    [0x6] = "grass",
    [0x7] = "ice",
    [0x8] = "volcano",
    [0x9] = "road_up",
    [0xA] = "road_down",
    [0xB] = "road_left",
    [0xC] = "road_right",
}

-- Which panels a row lists (`off_80E291C`, by the row's byte 1 / 4).
local ALL, ROW, FRONT, OWN = 0, 1, 2, 3

type Row = {
    -- The panel type the panels become.
    kind: number,
    collector: number,
    -- While the timer runs the panels show as the new type: every tick,
    -- or when flickering (the row's byte 2 set) only on ticks whose timer
    -- has bit 2.
    flicker: boolean,
    ticks: number,
    -- By side: what a listed panel's flags must have and must not.
    sides: { [number]: PanelCondition },
}

-- `byte_80E272C`, 20 bytes a kind: the type, the collector, the flicker
-- byte, the ticks, then side 0's require and forbid and side 1's (u32s).
-- Flags: 0x20 side 1's panel, 0x8000 missing, 0x200 road, 0x400 grass,
-- 0x800 ice, 0x1000 volcano, 0x2000 holy.
local ROWS: { [number]: Row } = {
    [0x00] = { kind = 0x2, collector = ALL, flicker = true, ticks = 0x40, sides = { [0] = { require = 0x0, forbid = 0x8020 }, [1] = { require = 0x20, forbid = 0x8000 } } },
    [0x01] = { kind = 0x8, collector = ALL, flicker = true, ticks = 0x40, sides = { [0] = { require = 0x0, forbid = 0x8000 }, [1] = { require = 0x0, forbid = 0x8000 } } },
    [0x02] = { kind = 0x7, collector = ALL, flicker = true, ticks = 0x40, sides = { [0] = { require = 0x0, forbid = 0x8000 }, [1] = { require = 0x0, forbid = 0x8000 } } },
    [0x03] = { kind = 0x6, collector = ALL, flicker = true, ticks = 0x40, sides = { [0] = { require = 0x0, forbid = 0x8000 }, [1] = { require = 0x0, forbid = 0x8000 } } },
    [0x04] = { kind = 0x5, collector = FRONT, flicker = true, ticks = 0x40, sides = { [0] = { require = 0x0, forbid = 0x8000 }, [1] = { require = 0x0, forbid = 0x8000 } } },
    [0x05] = { kind = 0x5, collector = ALL, flicker = true, ticks = 0x40, sides = { [0] = { require = 0x0, forbid = 0x8020 }, [1] = { require = 0x20, forbid = 0x8000 } } },
    [0x06] = { kind = 0x4, collector = OWN, flicker = true, ticks = 0x40, sides = { [0] = { require = 0x0, forbid = 0x8020 }, [1] = { require = 0x20, forbid = 0x8000 } } },
    [0x07] = { kind = 0x2, collector = FRONT, flicker = false, ticks = 0x40, sides = { [0] = { require = 0x0, forbid = 0x8000 }, [1] = { require = 0x0, forbid = 0x8000 } } },
    [0x08] = { kind = 0x2, collector = ROW, flicker = false, ticks = 0x40, sides = { [0] = { require = 0x0, forbid = 0x8000 }, [1] = { require = 0x0, forbid = 0x8000 } } },
    [0x09] = { kind = 0x2, collector = OWN, flicker = true, ticks = 0x1E, sides = { [0] = { require = 0x0, forbid = 0x8020 }, [1] = { require = 0x20, forbid = 0x8000 } } },
    [0x0A] = { kind = 0x2, collector = ALL, flicker = false, ticks = 0x1E, sides = { [0] = { require = 0x0, forbid = 0x8020 }, [1] = { require = 0x20, forbid = 0x8000 } } },
    [0x0B] = { kind = 0x4, collector = ALL, flicker = true, ticks = 0x1E, sides = { [0] = { require = 0x20, forbid = 0x8000 }, [1] = { require = 0x0, forbid = 0x8020 } } },
    [0x0C] = { kind = 0x2, collector = ALL, flicker = true, ticks = 0x1E, sides = { [0] = { require = 0x2000, forbid = 0x8020 }, [1] = { require = 0x2020, forbid = 0x8000 } } },
    [0x0D] = { kind = 0x2, collector = ALL, flicker = true, ticks = 0x1E, sides = { [0] = { require = 0x1000, forbid = 0x8000 }, [1] = { require = 0x1000, forbid = 0x8000 } } },
    [0x0E] = { kind = 0x2, collector = ALL, flicker = true, ticks = 0x1E, sides = { [0] = { require = 0x800, forbid = 0x8000 }, [1] = { require = 0x800, forbid = 0x8000 } } },
    [0x0F] = { kind = 0x2, collector = ALL, flicker = true, ticks = 0x1E, sides = { [0] = { require = 0x200, forbid = 0x8000 }, [1] = { require = 0x200, forbid = 0x8000 } } },
    [0x10] = { kind = 0x2, collector = ALL, flicker = true, ticks = 0x1E, sides = { [0] = { require = 0x400, forbid = 0x8000 }, [1] = { require = 0x400, forbid = 0x8000 } } },
    [0x11] = { kind = 0xB, collector = ROW, flicker = true, ticks = 0x40, sides = { [0] = { require = 0x20, forbid = 0x8000 }, [1] = { require = 0x0, forbid = 0x8020 } } },
    [0x12] = { kind = 0xC, collector = ROW, flicker = true, ticks = 0x40, sides = { [0] = { require = 0x20, forbid = 0x8000 }, [1] = { require = 0x0, forbid = 0x8020 } } },
}

-- The roads, which point the other way on side 1.
local ROAD_LEFT, ROAD_RIGHT = 0x11, 0x12

-- Sounds: the tick every fourth tick (not for kinds 7, 8 and 0xC), and the
-- change (0x90 for kinds 6 and 0xB, none for 7 and 8).
local TICK_SOUND, CHANGE_SOUND, POISON_SOUND = 0xA3, 0xA4, 0x90

-- `sub_80E2ACA`: a changer of kind `kind` on panel (`x`, `y`), on
-- `spawner`'s side and facing; `flag`: the object whose parameters it
-- clears when it is done (the game's pointer to its spawner's Param2, whose
-- word store clears all four), if any. (Its position is the spawner's
-- registers: X the panel's Y, Y and Z garbage, which nothing reads.)
function panel_changer.spawn(spawner: Object, x: number, y: number, kind: number, flag: Object?): Object?
    local o = battle.spawn_kind("panel-changer", Vec3.new(y, 0, 0), { kind, 0, 0, 0 })
    if o then
        o.panel_x, o.panel_y = x, y
        o.alliance, o.flip = spawner.alliance, spawner.flip
        o.related1 = flag
    end
    return o
end

local function row(me: Object): Row
    local s = me.state :: State
    local r = ROWS[s.kind]
    if r == nil then
        error(string.format("panel changer kind %#x reads past byte_80E272C", s.kind))
    end
    return r
end

-- `off_80E291C`: the panels the row picks, meeting the side's condition.
-- (`sub_80E29F8`: rows 3 to 1, columns 6 to 1; `sub_80E2A30`: its row,
-- columns 6 to 1; `sub_80E2A64`: the panel in front; `sub_80E2A9A`: its
-- own.)
local function collect(me: Object, r: Row): { number }
    local candidates: { number } = {}
    if r.collector == ALL then
        for y = 3, 1, -1 do
            for x = 6, 1, -1 do
                candidates[#candidates + 1] = panels.pack(x, y)
            end
        end
    elseif r.collector == ROW then
        for x = 6, 1, -1 do
            candidates[#candidates + 1] = panels.pack(x, me.panel_y)
        end
    elseif r.collector == FRONT then
        candidates[#candidates + 1] = panels.pack(me.panel_x + me.facing, me.panel_y)
    else
        candidates[#candidates + 1] = panels.pack(me.panel_x, me.panel_y)
    end
    local c = r.sides[me.alliance]
    local list: { number } = {}
    for _, p in candidates do
        if field.check(panels.x(p), panels.y(p), c.require, c.forbid) then
            table.insert(list, p)
        end
    end
    return list
end

-- Each listed panel, last to first.
local function each_panel(me: Object, f: (x: number, y: number) -> ())
    local s = me.state :: State
    for i = s.count, 1, -1 do
        local p = s.list[i]
        f(panels.x(p), panels.y(p))
    end
end

local function side_of(x: number, y: number): number
    local p = field.panel(x, y)
    return if p then p.alliance else 0
end

-- `sub_80E292C`: count down, showing the panels as their new type
-- meanwhile (`object_setPanelTypeBlink`: every tick, or when the row
-- flickers only on ticks whose timer has bit 2); at 0 change them, and go.
local function tick(me: Object)
    local r = row(me)
    local kind = me:param(1)
    local t = me.timer - 1
    me.timer = t
    local new = TYPES[r.kind]
    if t ~= 0 then
        if bit32.band(t, 3) == 0 and kind ~= 8 and kind ~= 7 and kind ~= 0xC then
            battle.play_sound(TICK_SOUND)
        end
        if r.flicker and not bit32.btest(t, 4) then
            return
        end
        each_panel(me, function(x, y)
            field.blink(x, y, new, side_of(x, y))
        end)
        return
    end
    each_panel(me, function(x, y)
        local side = side_of(x, y)
        field.set_type(x, y, new)
        field.blink(x, y, new, side)
    end)
    if kind ~= 8 and kind ~= 7 then
        battle.play_sound(if kind == 6 or kind == 0xB then POISON_SOUND else CHANGE_SOUND)
    end
    me:set_lifecycle("destroy")
end

-- `sub_80E28C8`: on side 1 the roads swap; list the panels; the timer; and
-- the first update at once.
local function init(me: Object)
    local kind = me:param(1)
    if me.alliance ~= 0 then
        if kind == ROAD_LEFT then
            kind = ROAD_RIGHT
        elseif kind == ROAD_RIGHT then
            kind = ROAD_LEFT
        end
        me:set_param(1, kind)
    end
    local s = me.state :: State
    s.kind = kind
    local r = row(me)
    local list = collect(me, r)
    for i, p in list do
        s.list[i] = p
    end
    s.count = #list
    me.timer = r.ticks
    me.lifecycle = "update"
    tick(me)
end

-- `sub_80E29E6`: clear the flag holder's parameters (the word store
-- through the pointer), and go.
local function finish(me: Object)
    local flag = me.related1
    if flag then
        for n = 1, 4 do
            flag:set_param(n, 0)
        end
    end
    me:free()
end

-- `sub_80E28A8`.
function panel_changer.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        finish(me)
    end
end

return panel_changer
