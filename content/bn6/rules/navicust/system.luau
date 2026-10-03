--!strict
-- BN6's NaviCust (docs/design/navicust.md): as the round is set up
-- (`round_setup`, before anything reads the stats and before the patch
-- cards, which apply to what the NaviCust made), a player's NaviCust
-- (`battle.navicust`: the programs placed on its grid) is compiled into the
-- navi's stats, as the original's reload does (`reloadCurNaviStatBoosts`'s
-- `sub_813C458`, for MegaMan alone):
--
-- 1. The bugs counted (`sub_813BBD4`), by the bug each program brings: a
--    plus part on the command line (`sub_813BC1C`) and a program off it
--    (`sub_813BC98`); for each program, each other program beside it (its
--    shape moved a cell left, right, up or down) of the same color, counted
--    for that one's bug (`sub_813BD24`); five colors on the grid, or more
--    (`sub_813BEA8`); a program on the board's frame (`sub_813C584`).
-- 2. The stats reset (`sub_8136C24`): the setup's are already the navi's
--    fresh stats with what the save keeps (nettai-match's `Side`), so no
--    work here.
-- 3. The programs' effects (`sub_813C684`): the programs on the command
--    line, right to left, of a group the first only (`exclusive`); the plus
--    parts off it, in the list's order; the plus parts on it, left to
--    right. The HP programs add up, and the maximum is the base and them.
-- 4. The bugs (`sub_813CBCC`), unless a program stopped them (BugStop): each
--    bug at its count's level, 1 to 3. Any bug sets the save's flag 0x1720,
--    the emotion window's glitch.
-- 5. The HP (`sub_803CE44`): the maximum again, and the HP the round starts
--    with (in the real world, the maximum).
--
-- A program's color, shape and whether it is a plus part are the engine's
-- (its definition's); its `bug`, its `exclusive` and its `effects` this
-- system's (./programs.luau). A program `anywhere` (no BN6 program is one)
-- works on the command line or off it and is never bugged by where it is.

local board = require("./board")
local programs = require("./programs")

local SIZE = board.SIZE
local CENTRE = SIZE // 2

-- Whether the cell of `rows` (a definition's shape: strings of `#` and
-- `.`) at (x, y) is filled.
local function filled(rows: { string }, x: number, y: number): boolean
    return string.sub(rows[y + 1], x + 1, x + 1) == "#"
end

-- A placed program's shape, compressed or not, turned (`sub_813B7A0`: as it
-- is, a quarter clockwise, a half, a quarter back): its cells by row.
local function shape(p: PlacedProgram): { { boolean } }
    local def = p.program
    local rows = if p.compressed and def.compressed ~= nil then def.compressed else def.shape
    local out: { { boolean } } = {}
    for y = 0, SIZE - 1 do
        out[y] = {}
        for x = 0, SIZE - 1 do
            out[y][x] = false
        end
    end
    local n = SIZE - 1
    for y = 0, n do
        for x = 0, n do
            if filled(rows, x, y) then
                local r = p.rotation % 4
                if r == 0 then
                    out[y][x] = true
                elseif r == 1 then
                    out[x][n - y] = true
                elseif r == 2 then
                    out[n - y][n - x] = true
                else
                    out[n - x][y] = true
                end
            end
        end
    end
    return out
end

-- The grid cells a shape covers with its centre at (x, y), in its rows'
-- order: { x, y } each.
local function cells(s: { { boolean } }, x: number, y: number): { { number } }
    local out: { { number } } = {}
    for j = 0, SIZE - 1 do
        for i = 0, SIZE - 1 do
            if s[j][i] then
                local cx, cy = x - CENTRE + i, y - CENTRE + j
                if cx >= 0 and cx < SIZE and cy >= 0 and cy < SIZE then
                    out[#out + 1] = { cx, cy }
                end
            end
        end
    end
    return out
end

local function at(grid: { [number]: number }, x: number, y: number): number?
    return grid[y * SIZE + x]
end

local function clamp_add(v: number, n: number, most: number): number
    return math.min(v + n, most)
end

-- What a program does (`navicust_jt_NCPs`' handler for it).
type Work = { hp: number, bug_stop: boolean }

local function apply(stats: Navi, e: NaviCustEffect, work: Work)
    local k = e.kind
    local n = e.amount or 0
    if k == "super_armor" then
        stats.super_armor = true
    elseif k == "custom" then
        stats.custom_level = clamp_add(stats.custom_level, n, 8)
    elseif k == "mega_folder" then
        stats.mega_level = clamp_add(stats.mega_level, n, 10)
    elseif k == "giga_folder" then
        stats.giga_level = clamp_add(stats.giga_level, n, 10)
    elseif k == "first_barrier" then
        stats.first_barrier = e.barrier
    elseif k == "back_special" then
        stats.back_special_weapon = e.weapon
    elseif k == "float_shoes" then
        stats.float_shoes = true
    elseif k == "air_shoes" then
        stats.air_shoes = true
    elseif k == "undershirt" then
        stats.undershirt = true
    elseif k == "chip_shuffle" then
        stats.chip_shuffle = true
    elseif k == "number_open" then
        stats.number_open = true
    elseif k == "collect" then
        stats.chip_drops = bit32.bor(stats.chip_drops, 2)
    elseif k == "attack" then
        stats.attack = clamp_add(stats.attack, n, 4)
    elseif k == "rapid" then
        stats.rapid = clamp_add(stats.rapid, n, 4)
    elseif k == "charge" then
        stats.charge = clamp_add(stats.charge, n, 4)
    elseif k == "attack_max" then
        stats.attack = 4
    elseif k == "rapid_max" then
        stats.rapid = 4
    elseif k == "charge_max" then
        stats.charge = 4
    elseif k == "bug_stop" then
        work.bug_stop = true
    elseif k == "rush" then
        stats.rush = true
    elseif k == "beat" then
        stats.beat = true
    elseif k == "tango" then
        stats.tango = true
    elseif k == "hp" then
        work.hp += n
    elseif k == "outside" then
        -- (A stat no netbattle reads.)
    else
        error(string.format("NaviCust effect %q is no kind of effect", tostring(k)))
    end
end

-- Each bug at its level, 1 to 3 (`byte_813CC18`'s routines).
local BUG_WORK: { [string]: (stats: Navi, level: number) -> () } = {
    step = function(stats, _)
        stats.step_bug = 1
    end,
    emotion = function(stats, _)
        stats.emotion_bug = 1
    end,
    panel = function(stats, level)
        stats.panel_trail = 3
        stats.panel_trail_level = level + 1
    end,
    custom = function(stats, level)
        stats.hand_shrink_turn = 5 - level
    end,
    encounter = function(stats, _)
        stats.encounters = 1
    end,
    result = function(stats, _)
        stats.chip_drops = 1
    end,
    buster = function(stats, level)
        stats.buster_blanks = ({ 6, 10, 13 })[level]
        stats.buster_charged = level
    end,
    support = function(stats, _)
        stats.support_bug = true
    end,
    hp = function(stats, level)
        stats.hp_drain = bit32.band(stats.hp_drain + level, 0xFF)
        stats.hit_status = 3
    end,
    unread = function(_, _)
        -- (NaviStats+0x62, which no battle reads.)
    end,
    status = function(stats, _)
        stats.battle_start_bug = 9
    end,
    ["status-strong"] = function(stats, _)
        stats.battle_start_bug = 10
    end,
}

-- `round_setup`: `side`'s NaviCust compiled into its stats, if the setup
-- gives one and the navi is MegaMan (the navi that changes form: the
-- original compiles the PET's own navi's, navi 0's).
local function compile(side: number)
    local nc = battle.navicust(side)
    if nc == nil then
        return
    end
    local stats = battle.navi(side)
    if stats.navi.forms == nil then
        return
    end
    local parts = nc.parts
    local layout = board.boards[nc.expansions + 1]
    if layout == nil then
        error(string.format("a NaviCust with %d expansions: BN6's board has %d sizes", nc.expansions, #board.boards))
    end

    -- The grid (`Unk200414c`, which placing a program writes:
    -- `sub_813B950`): each cell the program on it, by its place in the list.
    local shapes: { { { boolean } } } = {}
    local grid: { [number]: number } = {}
    for i, p in parts do
        shapes[i] = shape(p)
        for _, c in cells(shapes[i], p.x, p.y) do
            grid[c[2] * SIZE + c[1]] = i
        end
    end

    -- 1. The bugs counted (`sub_813BBD4`), by the bug each brings.
    local counts: { [string]: number } = {}
    local function count(bug: string?, n: number?)
        if bug ~= nil then
            counts[bug] = (counts[bug] or 0) + (n or 1)
        end
    end
    -- The command line, right to left (`sub_813BC1C`): each program on it
    -- once, at its rightmost cell; a plus part is bugged there (and works).
    local line = board.COMMAND_LINE
    local line_programs: { [number]: number } = {}
    local line_plus: { [number]: number } = {}
    local on_line: { [number]: boolean } = {}
    for x = SIZE - 1, 0, -1 do
        local i = at(grid, x, line)
        if i ~= nil and not on_line[i] then
            on_line[i] = true
            local def = parts[i].program
            if def.anywhere then
                line_plus[x] = i
            elseif def.plus then
                count(def.bug)
                line_plus[x] = i
            else
                line_programs[x] = i
            end
        end
    end
    -- Off the command line (`sub_813BC98`): a program is bugged there (and
    -- doesn't work); a plus part works.
    local off_plus: { [number]: boolean } = {}
    for i, p in parts do
        if not on_line[i] then
            local def = p.program
            if def.plus or def.anywhere then
                off_plus[i] = true
            else
                count(def.bug)
            end
        end
    end
    -- Beside one of its color (`sub_813BD24`): for each program, the
    -- programs its shape moved a cell meets (`sub_813BD90`; where a
    -- neighbour counts), each once; one of the same color counts for its
    -- own bug (`sub_813BE38`).
    for i, p in parts do
        local met: { number } = {}
        for _, d in { { -1, 0 }, { 1, 0 }, { 0, -1 }, { 0, 1 } } do
            for _, c in cells(shapes[i], p.x + d[1], p.y + d[2]) do
                local q = at(grid, c[1], c[2])
                if q ~= nil and q ~= i and table.find(met, q) == nil then
                    if string.sub(board.NEIGHBOURS[c[2] + 1], c[1] + 1, c[1] + 1) ~= "x" then
                        table.insert(met, q)
                    end
                end
            end
        end
        for _, q in met do
            if parts[q].color == p.color then
                count(parts[q].program.bug)
            end
        end
    end
    -- The colors (`sub_813BEA8`): five, the status bug; more, the strong one
    -- twice.
    local colors: { string } = {}
    for _, p in parts do
        if table.find(colors, p.color) == nil then
            table.insert(colors, p.color)
        end
    end
    if #colors == 5 then
        count("status")
    elseif #colors > 5 then
        count("status-strong", 2)
    end
    -- On the frame (`sub_813C584`): each program covering a cell of it,
    -- once, in the list's order.
    local on_frame: { [number]: boolean } = {}
    for y = 0, SIZE - 1 do
        for x = 0, SIZE - 1 do
            local i = at(grid, x, y)
            if i ~= nil and string.sub(layout[y + 1], x + 1, x + 1) == "f" then
                on_frame[i] = true
            end
        end
    end
    for i, p in parts do
        if on_frame[i] then
            count(p.program.bug)
        end
    end

    -- 3. The programs' effects (`sub_813C684`).
    local work: Work = { hp = 0, bug_stop = false }
    local function run(def: NaviCustProgram)
        for _, e in def.effects or {} do
            apply(stats, e, work)
        end
    end
    -- The programs on the command line, right to left: of an exclusive
    -- group, the rightmost only.
    local groups: { [number]: string } = {}
    for x = SIZE - 1, 0, -1 do
        local i = line_programs[x]
        if i ~= nil then
            local def = parts[i].program
            local taken = false
            if def.exclusive ~= nil then
                groups[x] = def.exclusive
                for x2 = x + 1, SIZE - 1 do
                    if groups[x2] == def.exclusive then
                        taken = true
                    end
                end
            end
            if not taken then
                run(def)
            end
        end
    end
    -- The plus parts off it, in the list's order; on it, left to right.
    for i, p in parts do
        if off_plus[i] then
            run(p.program)
        end
    end
    for x = 0, SIZE - 1 do
        local i = line_plus[x]
        if i ~= nil then
            run(parts[i].program)
        end
    end
    stats.max_hp = stats.max_base_hp + work.hp

    -- 4. The bugs, unless stopped (`sub_813CBCC`).
    local bugged = false
    if not work.bug_stop then
        for _, bug in programs.BUGS do
            local level = math.min(counts[bug] or 0, 3)
            if level > 0 then
                BUG_WORK[bug](stats, level)
                bugged = true
            end
        end
    end
    battle.set_emotion_window_glitch(side, bugged)

    -- 5. The HP (`sub_803CE44`): the round starts with the maximum.
    stats.max_hp = stats.max_base_hp + work.hp
    stats.hp = stats.max_hp
end

return define.system {
    id = "bn6:navicust",
    hooks = { round_setup = compile },
}
