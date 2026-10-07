--!strict
-- EXE4's wind (effect object #0x41, 0x080E6820; EXE6's `sub_80E5244` by its place, the navis' AI wind, which no EXE6
-- stage places): what its wind stages (the actor lists' entry type 6, 0x080068DE: records 88 to 95) put off the field
-- for each side. Unseen, with a sound as it starts, it blows a gust (objects/gust) from the other side's first panel of
-- a row toward that side, the rows by turns (2, 1, 3, ...), 13 ticks after the last, while its side has no wind of the
-- battle's registry (a chip's: a stage's waits).
--
-- It tracks its gusts in two slots (+0x60, +0x61) and would wait with both full; but the gust spawner's word store
-- (objects/gust) only ever fills the first, so while its first gust is out each gust goes into the second slot, which
-- reads 0, and ends at once unseen.
--
-- Its third parameter is 1 from a stage (0x080E6932), whose wind waits on its side's registered one and leaves the
-- registry alone; the navis' AI wind (0x080E68F8, from callers not ported) has 0, registers itself as its side's wind,
-- and its end clears that.

local gust = require("./gust")
local panels = require("@exelib/panels")

local wind = {}

export type State = {
    -- Its gusts' slots (the original's +0x60 and +0x61).
    gusts: { number },
    -- The row the next gust is for, counting down from 3 by turns (+0x6C).
    row: number,
    -- How its gusts blow (its first parameter: bit 0 pulls, bit 7 unseen).
    pulls: boolean,
    unseen: boolean,
    -- A stage's (its third parameter): it waits while its side has a registered wind.
    waits: boolean,
}

local function state(me: Object): State
    return me.state :: State
end

-- Its start (0x080E6846: 0x121).
local START = asset.sound("wind")
-- The ticks it waits after a gust (Timer2).
local WAIT = 12
-- The panel a gust starts on (0x080E69AC, by side: require, forbid): the other side's.
local PANELS = {
    [0] = { 0x20, 0x00 },
    [1] = { 0x00, 0x20 },
}
-- A panel with a neutral object (an obstacle's body).
local NEUTRAL = 0x800000

-- 0x080E6950: the panel of row `y` a gust starts on, 0 for none: the other side's first panel from the far edge
-- (`object_getClosestPanelMatchingRowFiltered` for the other side), none where a neutral object stands; a wind with a
-- first parameter (one that pulls or is unseen) walks on along its side's forward to the last panel before a neutral
-- object or the field's edge.
local function start_panel(me: Object, s: State, y: number): number
    local side = me.alliance
    local rule = PANELS[side]
    if rule == nil then
        error(string.format("wind side %d reads past its panel table (0x080E69AC)", side))
    end
    local x = panels.closest_in_row(bit32.bxor(side, 1), y, rule[1], rule[2])
    if bit32.btest(field.flags(x, y), NEUTRAL) then
        return 0
    end
    if not (s.pulls or s.unseen) then
        return x
    end
    local dir = panels.side_direction(side)
    repeat
        x += dir
    until x < 1 or x > 6 or bit32.btest(field.flags(x, y), NEUTRAL)
    return x - dir
end

-- 0x080E687C: a gust when its side's registered wind (a stage's) and its slots let it, for the next row by turns,
-- tracked in its first free slot; then the wait.
local function blow(me: Object)
    local s = state(me)
    if s.waits and battle.wind(me.alliance) ~= nil then
        return
    end
    if s.gusts[1] ~= 0 and s.gusts[2] ~= 0 then
        return
    end
    local slot = if s.gusts[1] == 0 then 0 else 1
    s.row -= 1
    if s.row == 0 then
        s.row = 3
    end
    local x = start_panel(me, s, s.row)
    if x == 0 then
        return
    end
    gust.spawn(x, s.row, me.alliance, { pulls = s.pulls, unseen = s.unseen }, 0, me, slot)
    me.timer2 = WAIT
    me.phase = 4
end

-- 0x080E68D0: the wait.
local function wait(me: Object)
    me.timer2 -= 1
    if me.timer2 == 0 then
        me.phase = 0
    end
end

-- 0x080E6858: by its phase.
local function tick(me: Object)
    local p = me.phase
    if p == 0 then
        blow(me)
    elseif p == 4 then
        wait(me)
    else
        error(string.format("wind phase %#x reads past its table (0x080E6874)", p))
    end
end

wind.kind = new.kind {
    id = "wind",
    pool = "effect",
    state = { gusts = "u8[2]", row = "u8", pulls = "bool", unseen = "bool", waits = "bool" },
    -- 0x080E6820.
    update = function(me: Object)
        local l = me.lifecycle
        local s = state(me)
        if l == "init" then
            -- 0x080E6840: the row count, the sound, and its first update at once.
            s.row = 3
            battle.play_sound(START)
            me.lifecycle = "update"
            tick(me)
        elseif l == "update" then
            tick(me)
        else
            -- 0x080E68E0: one that registered itself is no side's wind; its slots empty.
            if not s.waits then
                battle.clear_wind(me)
            end
            s.gusts[1], s.gusts[2] = 0, 0
            me:free()
        end
    end,
    -- 0x080068DE, 0x080E6932: a wind stage's wind (an actor list's entry type 6) of the entry's side (it has no
    -- panel); its first parameter the entry's first byte, 0 in every record (it pushes, seen); not running while
    -- dimmed. (Its position is the spawner's registers.)
    place = function(spec: PlaceSpec): Object?
        local o = battle.spawn(wind.kind, Vec3.zero)
        if not o then
            return nil
        end
        local s = state(o)
        s.waits = true
        o.alliance = spec.side
        o.run_while_dimmed = false
        return o
    end,
}

return wind
