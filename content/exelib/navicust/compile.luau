--!strict
-- The NaviCust compile EXE5's and EXE6's share (docs/design/navicust.md §3, §4): as the round is set up (a game's
-- rules/navicust's `round_setup`, before anything reads the stats and before the patch cards, which apply to what
-- the NaviCust made), a player's NaviCust (`battle.navicust`: the programs placed on its grid) is compiled into the
-- navi's stats, as the original's reload does (EXE6's `reloadCurNaviStatBoosts`'s `sub_813C458`, EXE5's 0x0813FA10,
-- the same routines), for MegaMan alone:
--
-- 1. The bugs counted (EXE6's `sub_813BBD4`, EXE5's 0x0813F310), by the bug each program brings: a plus part on the
--    command line (`sub_813BC1C`) and a program off it (`sub_813BC98`); for each program, each other program beside
--    it (its shape moved a cell left, right, up or down, on a cell where a neighbor counts) of the same color,
--    counted for that one's bug (`sub_813BD24`); what the game counts besides (`sub_813BE8C`: EXE5's HubBatc); five
--    colors on the grid, or more (`sub_813BEA8`); a program on the board's frame (EXE6's `sub_813C584`; EXE5's board
--    has none).
-- 2. The stats reset (`sub_8136C24`): the setup's are already the navi's fresh stats with what the save keeps
--    (nettai-match's `Side`), so no work here.
-- 3. The programs' effects (`sub_813C684`): the programs on the command line, right to left, of a group the first
--    only (`exclusive`); the plus parts off it, in the list's order; the plus parts on it, left to right. The HP
--    programs add up, and the maximum is the base and them.
-- 4. The bugs (`sub_813CBCC`), unless a program stopped them (BugStop): each bug at its count's level, 1 to 3, as the
--    game's table says. Any bug sets the save's flag (EXE6's 0x1720, EXE5's 0x10C1), the emotion window's glitch.
-- 5. The HP (`sub_803CE44`): the maximum again, and the HP the round starts with, the maximum (the original's in
--    the real world; EXE5's other ending, in the cyberworld, is none a link battle can tell: its rules say why).
--
-- A program's color, shape and whether it is a plus part are the engine's (its definition's); its `bug`, its
-- `exclusive` and its `effects` the rules' (./effects). A program `anywhere` (no EXE5 or EXE6 program is one) works on
-- the command line or off it and is never bugged by where it is.
--
-- A side without a NaviCust has its stats as a compile left them (a recording's, a match's set directly, a link
-- navi's): no setup gives the glitch, so its flag is what those stats say, set when they carry a bug (`stats_bugged`:
-- any stat a bug's routine writes). Every recorded MegaMan with the flag has such a stat but EXE5's HubBatc alone,
-- whose bug halves the HP programs and writes none.

local compile = {}

-- A game's board and what its compile is made of besides the shared routines: `NaviCustBoard`, `NaviCustGame`
-- (common/types.d.luau). EXE5's HubBatc counts its own bug once more (`extra`, 0x0813F5A8); EXE6's flag 0x1720 is
-- the emotion window's glitch (`glitch`), EXE5's 0x10C1 one no battle reads.

-- Whether the cell of `rows` (a definition's shape: strings of `#` and `.`) at (x, y) is filled.
local function filled(rows: { string }, x: number, y: number): boolean
    return string.sub(rows[y + 1], x + 1, x + 1) == "#"
end

-- A placed program's shape, compressed or not, turned (`sub_813B7A0`: as it is, a quarter clockwise, a half, a
-- quarter back): its cells by row.
local function shape(p: PlacedProgram, size: number): { { boolean } }
    local def = p.program
    local rows = if p.compressed and def.compressed ~= nil then def.compressed else def.shape
    local out: { { boolean } } = {}
    for y = 0, size - 1 do
        out[y] = {}
        for x = 0, size - 1 do
            out[y][x] = false
        end
    end
    local n = size - 1
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

-- The grid cells a shape covers with its center at (x, y), in its rows' order: { x, y } each.
local function cells(s: { { boolean } }, x: number, y: number, size: number): { { number } }
    local center = size // 2
    local out: { { number } } = {}
    for j = 0, size - 1 do
        for i = 0, size - 1 do
            if s[j][i] then
                local cx, cy = x - center + i, y - center + j
                if cx >= 0 and cx < size and cy >= 0 and cy < size then
                    out[#out + 1] = { cx, cy }
                end
            end
        end
    end
    return out
end

-- What is wrong with a NaviCust (the game's setup's `navicust_expansions` and `navicust_programs`) on `game`'s board,
-- each said with `problem(text, field, entry)` (the setup field, and a program's place in the list, from 1): a board of its expansions; each
-- program in one of its colors (its definition's `colors`), turned 0 to 3 quarters, on the board (EXE6's
-- `sub_813BB00`: every cell it covers is a cell of the board or its frame, and not all of them the frame), over no
-- other (`sub_813BB68`); the copies of a program in one color all compressed or none (the save keeps it by program and
-- color: EXE6's event flag 0x2660 + the part id). `compiles`: the side's navi compiles one (MegaMan, the navi that
-- changes form); another's programs are refused.
function compile.check(
    game: NaviCustGame,
    expansions: number?,
    parts: { PlacedProgram },
    compiles: boolean,
    problem: (text: string, field: string, entry: number?) -> ()
)
    if expansions == nil then
        return
    end
    local board = game.board
    local size = board.SIZE
    if #parts > 0 and not compiles then
        problem("a NaviCust, but the navi's stats aren't a NaviCust's (only MegaMan's compiles)", "navicust_programs", nil)
        return
    end
    local layout = board.boards[expansions + 1]
    if layout == nil then
        problem(string.format("a NaviCust with %d expansions: the game's board has %d sizes", expansions, #board.boards), "navicust_expansions", nil)
        return
    end
    local grid: { [number]: number } = {}
    local compression: { [string]: boolean } = {}
    for i, p in parts do
        local def = p.program
        local name = battle.name(def)
        local what = string.format("navicust program %d (%s)", i, name)
        if table.find(def.colors, p.color) == nil then
            problem(string.format("%s: a color it doesn't come in", what), "navicust_programs", i)
        end
        if p.rotation > 3 then
            problem(string.format("%s: turned %d quarters (0 to 3)", what, p.rotation), "navicust_programs", i)
        end
        -- Its cells, the grid's or not.
        local s = shape(p, size)
        local center = size // 2
        local covered: { { number } } = {}
        for j = 0, size - 1 do
            for k = 0, size - 1 do
                if s[j][k] then
                    table.insert(covered, { p.x - center + k, p.y - center + j })
                end
            end
        end
        local fits, on = true, false
        for _, c in covered do
            local cell = if c[1] >= 0 and c[1] < size and c[2] >= 0 and c[2] < size
                then string.sub(layout[c[2] + 1], c[1] + 1, c[1] + 1)
                else "."
            if cell == "." then
                fits = false
            elseif cell == "o" then
                on = true
            end
        end
        if not (fits and on) then
            problem(string.format("%s at (%d, %d): off the board", what, p.x, p.y), "navicust_programs", i)
        else
            for _, c in covered do
                local at = c[2] * size + c[1]
                local other = grid[at]
                if other ~= nil then
                    problem(string.format("%s is over program %d", what, other), "navicust_programs", i)
                    break
                end
                grid[at] = i
            end
        end
        local key = (def.id :: string) .. "/" .. p.color
        local seen = compression[key]
        if seen == nil then
            compression[key] = p.compressed
        elseif seen ~= p.compressed then
            problem(string.format("%s: compressed and not (a save compresses every copy of a program in one color)", what), "navicust_programs", i)
        end
    end
end

local function clamp_add(v: number, n: number, most: number): number
    return math.min(v + n, most)
end

-- What a program does (the game's handler for it).
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
    elseif k == "soul_time" then
        stats.soul_turn_bonus = clamp_add(stats.soul_turn_bonus, n, 6)
    elseif k == "hp" then
        work.hp += n
    elseif k == "outside" then
        -- (A stat no netbattle reads.)
    else
        error(string.format("NaviCust effect %q is no kind of effect", tostring(k)))
    end
end

-- Whether `stats` carry a NaviCust bug: any stat a bug's routine writes (the games' `bug_work`), or the patch cards'
-- (the custom screen's damage and drain).
local function stats_bugged(stats: Navi): boolean
    return stats.step_bug == 1
        or stats.emotion_bug ~= 0
        or stats.panel_trail_level ~= 0
        or stats.hand_shrink_turn ~= 0
        or stats.custom_damage ~= 0
        or stats.custom_drain ~= 0
        or stats.encounters == 1
        or stats.chip_drops == 1
        or stats.buster_blanks ~= 0
        or stats.support_bug
        or stats.hp_drain ~= 0
        or stats.hit_status ~= 0
        or stats.battle_start_bug ~= 0
end

-- `round_setup`: `side`'s NaviCust compiled into its stats by `game`'s rules, if the setup gives one (`expansions`,
-- how far its board has been expanded, none without one; `parts`, its programs in its list's order: the game's setup's
-- `navicust_expansions` and `navicust_programs`) and the navi is MegaMan (the navi that changes form: the original
-- compiles the PET's own navi's, navi 0's). Whether it was. Without one, the stats are as a compile left them, and the
-- glitch is as their bugs say.
function compile.run(side: number, game: NaviCustGame, expansions: number?, parts: { PlacedProgram }): boolean
    if expansions == nil then
        if game.glitch then
            battle.set_emotion_window_glitch(side, stats_bugged(battle.navi(side)))
        end
        return false
    end
    local stats = battle.navi(side)
    if stats.navi.forms == nil then
        return false
    end
    local board = game.board
    local size = board.SIZE
    local layout = board.boards[expansions + 1]
    if layout == nil then
        error(string.format("a NaviCust with %d expansions: %s's board has %d sizes", expansions, game.name, #board.boards))
    end
    local function at(grid: { [number]: number }, x: number, y: number): number?
        return grid[y * size + x]
    end

    -- The grid (EXE6's `Unk200414c`, EXE5's 0x0200xxxx grid, which placing a program writes: `sub_813B950`): each cell
    -- the program on it, by its place in the list.
    local shapes: { { { boolean } } } = {}
    local grid: { [number]: number } = {}
    for i, p in parts do
        shapes[i] = shape(p, size)
        for _, c in cells(shapes[i], p.x, p.y, size) do
            grid[c[2] * size + c[1]] = i
        end
    end

    -- 1. The bugs counted (`sub_813BBD4`), by the bug each brings.
    local counts: { [string]: number } = {}
    local function count(bug: string?, n: number?)
        if bug ~= nil then
            counts[bug] = (counts[bug] or 0) + (n or 1)
        end
    end
    -- The command line, right to left (`sub_813BC1C`): each program on it once, at its rightmost cell; a plus part
    -- is bugged there (and works).
    local line = board.COMMAND_LINE
    local line_programs: { [number]: number } = {}
    local line_plus: { [number]: number } = {}
    local on_line: { [number]: boolean } = {}
    for x = size - 1, 0, -1 do
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
    -- Off the command line (`sub_813BC98`): a program is bugged there (and doesn't work); a plus part works.
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
    -- Beside one of its color (`sub_813BD24`): for each program, the programs its shape moved a cell meets
    -- (`sub_813BD90`; where a neighbor counts), each once; one of the same color counts for its own bug
    -- (`sub_813BE38`).
    for i, p in parts do
        local met: { number } = {}
        for _, d in { { -1, 0 }, { 1, 0 }, { 0, -1 }, { 0, 1 } } do
            for _, c in cells(shapes[i], p.x + d[1], p.y + d[2], size) do
                local q = at(grid, c[1], c[2])
                if q ~= nil and q ~= i and table.find(met, q) == nil then
                    if string.sub(board.NEIGHBORS[c[2] + 1], c[1] + 1, c[1] + 1) ~= "x" then
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
    -- What the game counts besides (`sub_813BE8C`).
    if game.extra ~= nil then
        for _, p in parts do
            game.extra(p, count)
        end
    end
    -- The colors (`sub_813BEA8`): five, the status bug; more, the strong one twice.
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
    -- On the frame (`sub_813C584`): each program covering a cell of it, once, in the list's order.
    local on_frame: { [number]: boolean } = {}
    for y = 0, size - 1 do
        for x = 0, size - 1 do
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
    -- The programs on the command line, right to left: of an exclusive group, the rightmost only.
    local groups: { [number]: string } = {}
    for x = size - 1, 0, -1 do
        local i = line_programs[x]
        if i ~= nil then
            local def = parts[i].program
            local taken = false
            if def.exclusive ~= nil then
                groups[x] = def.exclusive
                for x2 = x + 1, size - 1 do
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
    for x = 0, size - 1 do
        local i = line_plus[x]
        if i ~= nil then
            run(parts[i].program)
        end
    end
    stats.max_hp = stats.max_base_hp + work.hp

    -- 4. The bugs, unless stopped (`sub_813CBCC`; EXE5's 0x0813FA48 asks its patch cards first).
    local bugged = false
    local stopped = work.bug_stop
    if game.bug_stop ~= nil then
        local o = game.bug_stop(side)
        if o ~= nil then
            stopped = o
        end
    end
    if not stopped then
        for _, bug in game.bugs do
            local level = math.min(counts[bug] or 0, 3)
            if level > 0 then
                game.bug_work[bug](stats, level, work.hp)
                bugged = true
            end
        end
    end
    if game.glitch then
        battle.set_emotion_window_glitch(side, bugged)
    end

    -- 5. The HP (EXE6's `sub_803CE44`, EXE5's 0x0803C13C): the maximum again, and the round starts with it.
    stats.max_hp = stats.max_base_hp + work.hp
    stats.hp = stats.max_hp
    return true
end

return compile
