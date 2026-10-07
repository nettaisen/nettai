--!strict
-- EXE4's NaviCust (docs/design/exe4-map.md §8): as the round is set up (`round_setup`, after the save's part and
-- before anything reads the stats), a player's NaviCust (the setup's `navicust_programs`: the programs placed on its
-- grid) is compiled into MegaMan's stats, as the PET's reload does (0x08035130). EXE4's own routines, which EXE5's
-- and EXE6's (@exelib/navicust/compile) grew from and differ from in most steps: the neighbors met by the shapes
-- uncompressed, a program on the line skipped by its part (not an exclusive group), the colors bringing two bugs, the
-- bugs run twice around the Mod Cards, BugStop's dropped counts, the HP left as it was. Its 5x5 board is in the middle
-- of the engine's 7x7 grid, its command line the third row (the engine's fourth); no frame and no expansions.
--
-- 1. The analysis (0x08047344): the bugs counted by the bug each program brings: a plus part on the command line
--    (0x080473D4, the line right to left, each program at its rightmost cell); a program off it (0x08047458); for each
--    program, each other one beside it (its shape, uncompressed whether or not it is compressed, moved a cell left,
--    right, up or down, meeting another on the grid: 0x080474CC) of its color (0x0804753C); HubBatc, once more
--    (0x08047614); more than four colors (0x08047644: five bring `move` and `custom` once, more twice).
-- 2. The stats reset (0x08036CC0: the navi's fresh stats keeping the mood, +0x20, +0x29 and the light/dark value):
--    the setup's are already that.
-- 3. The programs' effects (0x08041974): the programs on the command line, right to left, one in a part (program and
--    color) already run to the right skipped; the plus parts off it, in the list's order; on it, left to right. The
--    HP programs add up, and the maximum is the base and them (0x08042FD0).
-- 4. The bugs (0x08042A94): unless BugStop (+0x1D) is on, which drops the counts instead (0x080477DC), each bug at its
--    count's level, 1 to 3 (0x08047694), by the table 0x08042B18.
-- 5. The Mod Cards (0x08041E50, then each card from the last slot to the first: 0x08041E6A), which raise the stats,
--    the HP programs' sum (the maximum again with it) and the bugs' counts (./mod_cards: `cards`).
-- 6. The bugs again, all of them, whatever BugStop (0x08042A58).
--
-- The PET leaves the HP (+0x30) as it was, the save's: the HP programs raise the maximum alone.

local programs = require("@self/programs")

-- HubBatc (program 23), which bugs itself.
local HUBBATC = "hubbatc"

-- The engine's grid is this many cells a side; EXE4's is the 5x5 in its middle, the command line its third row.
local SIZE = 7
local COMMAND_LINE = 3
local BOARD = {
    ".......",
    ".ooooo.",
    ".ooooo.",
    ".ooooo.",
    ".ooooo.",
    ".ooooo.",
    ".......",
}

-- What a program's handler and a bug's routine work on besides the stats: the HP programs' sum, BugStop.
export type Work = { hp: number, bug_stop: boolean }

-- What the Mod Cards do between the bugs' passes (./mod_cards): the side's cards applied to its stats, `count`
-- raising a bug's count.
export type Cards = (side: number, stats: Navi, work: Work, count: (bug: string, n: number) -> ()) -> ()

local function filled(rows: { string }, x: number, y: number): boolean
    return string.sub(rows[y + 1], x + 1, x + 1) == "#"
end

-- A placed program's shape (compressed or not), turned `rotation` quarters clockwise (0x08047006's four ways): its
-- cells by row, on the engine's grid's size.
local function shape(def: NaviCustProgram, rotation: number, compressed: boolean): { { boolean } }
    local rows = if compressed and def.compressed ~= nil then def.compressed else def.shape
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
                local r = rotation % 4
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

-- The grid cells a shape covers with its center at (x, y), on the board: { x, y } each.
local function cells(s: { { boolean } }, x: number, y: number): { { number } }
    local center = SIZE // 2
    local out: { { number } } = {}
    for j = 0, SIZE - 1 do
        for i = 0, SIZE - 1 do
            if s[j][i] then
                local cx, cy = x - center + i, y - center + j
                if cx >= 0 and cx < SIZE and cy >= 0 and cy < SIZE and string.sub(BOARD[cy + 1], cx + 1, cx + 1) == "o" then
                    table.insert(out, { cx, cy })
                end
            end
        end
    end
    return out
end

local function clamp_add(v: number, n: number, most: number): number
    return math.min(v + n, most)
end

-- A program's handler (0x08041A50's).
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
    elseif k == "weapon_level" then
        stats.weapon_level = clamp_add(stats.weapon_level, n, 2)
    elseif k == "bug_stop" then
        work.bug_stop = true
    elseif k == "rush" or k == "beat" or k == "tango" then
        -- (+0x18 holds one: the last placed.)
        stats.rush, stats.beat, stats.tango, stats.support_bug = k == "rush", k == "beat", k == "tango", false
    elseif k == "hp" then
        work.hp += n
    elseif k == "outside" then
        -- (A stat no netbattle reads.)
    else
        error(string.format("NaviCust effect %q is no kind of EXE4's", tostring(k)))
    end
end

-- Each bug at its level, 1 to 3 (0x08042B18's routines).
local BUG_WORK: { [string]: (stats: Navi, level: number, work: Work) -> () } = {
    move = function(stats, level)
        stats.move_bug = ({ 0xFF, 0x10, 0x20 })[level]
    end,
    hp = function(stats, level)
        stats.hp_drain = ({ 10, 6, 3 })[level]
    end,
    custom = function(stats, level)
        stats.custom_drain = ({ 6, 4, 3 })[level]
    end,
    panel = function(stats, level)
        stats.panel_trail = ({ 3, 1, 4 })[level]
    end,
    encounter = function(_stats, _level)
        -- (+0x15 = 1: no netbattle reads it.)
    end,
    buster = function(stats, level)
        stats.buster_blanks = ({ 4, 8, 10 })[level]
    end,
    support = function(stats, _level)
        stats.rush, stats.beat, stats.tango, stats.support_bug = false, false, false, true
    end,
    result = function(_stats, _level)
        -- (+0x19 = 1: no netbattle reads it.)
    end,
    hub = function(stats, _level, work)
        stats.max_hp = bit32.band(math.max(bit32.rshift(stats.max_base_hp, 1), 1) + work.hp, 0xFFFF)
    end,
}

-- The setup's programs as placed parts.
type Placed = { program: NaviCustProgram, color: string, x: number, y: number, rotation: number, compressed: boolean }

-- `round_setup`: `side`'s NaviCust compiled into its stats, and its Mod Cards (`cards`) applied, if its navi is
-- MegaMan (the navi that changes form: the PET compiles its own navi's).
local function run(side: number, cards: Cards?)
    local parts = rules.setup_of(side).navicust_programs :: { Placed }
    local stats = battle.navi(side)
    if stats.navi.forms == nil then
        return
    end
    local function at(grid: { [number]: number }, x: number, y: number): number?
        return grid[y * SIZE + x]
    end
    -- The grid (the save's 0x4540, which placing a program writes): each cell the program on it, by its place in the
    -- list; the shapes as placed, compressed or not.
    local grid: { [number]: number } = {}
    for i, p in parts do
        for _, c in cells(shape(p.program, p.rotation, p.compressed), p.x, p.y) do
            grid[c[2] * SIZE + c[1]] = i
        end
    end

    -- 1. The bugs counted (0x08047344), by the bug each brings.
    local counts: { [string]: number } = {}
    local function count(bug: string?, n: number?)
        if bug ~= nil then
            counts[bug] = (counts[bug] or 0) + (n or 1)
        end
    end
    -- The command line, right to left (0x080473D4): each program on it once, at its rightmost cell; a plus part is
    -- bugged there (and works).
    local line_programs: { [number]: number } = {}
    local line_plus: { [number]: number } = {}
    local on_line: { [number]: boolean } = {}
    for x = SIZE - 1, 0, -1 do
        local i = at(grid, x, COMMAND_LINE)
        if i ~= nil and not on_line[i] then
            on_line[i] = true
            local def = parts[i].program
            if def.plus then
                count(def.bug)
                line_plus[x] = i
            else
                line_programs[x] = i
            end
        end
    end
    -- Off the command line (0x08047458): a program is bugged there (and doesn't work); a plus part works.
    local off_plus: { [number]: boolean } = {}
    for i, p in parts do
        if not on_line[i] then
            if p.program.plus then
                off_plus[i] = true
            else
                count(p.program.bug)
            end
        end
    end
    -- Beside one of its color (0x080474CC): for each program, the programs its uncompressed shape moved a cell meets,
    -- each once; one of the same color counts for its own bug (0x0804753C).
    for i, p in parts do
        local met: { number } = {}
        local s = shape(p.program, p.rotation, false)
        for _, d in { { -1, 0 }, { 1, 0 }, { 0, -1 }, { 0, 1 } } do
            for _, c in cells(s, p.x + d[1], p.y + d[2]) do
                local q = at(grid, c[1], c[2])
                if q ~= nil and q ~= i and table.find(met, q) == nil then
                    table.insert(met, q)
                end
            end
        end
        for _, q in met do
            if parts[q].color == p.color then
                count(parts[q].program.bug)
            end
        end
    end
    -- HubBatc counts its own bug once more (0x08047614: part ids 0x5C to 0x5F, program 23).
    for _, p in parts do
        if p.program.id == HUBBATC then
            count(p.program.bug)
        end
    end
    -- The colors (0x08047644: the game state's +0x14, the color bar's count): five bring `move` and `custom` once,
    -- more twice.
    local colors: { string } = {}
    for _, p in parts do
        if table.find(colors, p.color) == nil then
            table.insert(colors, p.color)
        end
    end
    if #colors == 5 then
        count("move")
        count("custom")
    elseif #colors > 5 then
        count("move", 2)
        count("custom", 2)
    end

    -- 3. The programs' effects (0x08041974).
    local work: Work = { hp = 0, bug_stop = false }
    local function run_program(def: NaviCustProgram)
        for _, e in def.effects or {} do
            apply(stats, e, work)
        end
    end
    -- The programs on the command line, right to left: a part (program and color) already run to its right isn't.
    local ran: { string } = {}
    for x = SIZE - 1, 0, -1 do
        local i = line_programs[x]
        if i ~= nil then
            local p = parts[i]
            local part = (p.program.id :: string) .. "/" .. p.color
            if table.find(ran, part) == nil then
                table.insert(ran, part)
                run_program(p.program)
            end
        end
    end
    -- The plus parts off it, in the list's order; on it, left to right.
    for i, p in parts do
        if off_plus[i] then
            run_program(p.program)
        end
    end
    for x = 0, SIZE - 1 do
        local i = line_plus[x]
        if i ~= nil then
            run_program(parts[i].program)
        end
    end
    stats.max_hp = bit32.band(stats.max_base_hp + work.hp, 0xFFFF)

    -- 4. The bugs, unless BugStop drops their counts (0x08042A94).
    local function bugs()
        for _, bug in programs.BUGS do
            local level = math.min(counts[bug] or 0, 3)
            if level > 0 then
                BUG_WORK[bug](stats, level, work)
            end
        end
    end
    if work.bug_stop then
        counts = {}
    else
        bugs()
    end

    -- 5. The Mod Cards, then 6. the bugs again.
    if cards ~= nil then
        cards(side, stats, work, function(bug: string, n: number)
            count(bug, n)
        end)
    end
    bugs()
end

-- `validate`'s part: each program in one of its colors, turned 0 to 3 quarters, every cell it covers on the board, over
-- no other.
local function validate(side: number, problem: (text: string, field: string?, entry: number?) -> ())
    local parts = rules.setup_of(side).navicust_programs :: { Placed }
    if #parts > 0 and battle.navi(side).navi.forms == nil then
        problem("a NaviCust, but the navi's stats aren't a NaviCust's (only MegaMan's compiles)", "navicust_programs", nil)
        return
    end
    local grid: { [number]: number } = {}
    for i, p in parts do
        local def = p.program
        local what = string.format("navicust program %d (%s)", i, battle.name(def))
        if table.find(def.colors, p.color) == nil then
            problem(string.format("%s: a color it doesn't come in", what), "navicust_programs", i)
        end
        if p.rotation > 3 then
            problem(string.format("%s: turned %d quarters (0 to 3)", what, p.rotation), "navicust_programs", i)
        end
        local s = shape(def, p.rotation, p.compressed)
        local center = SIZE // 2
        local covered = 0
        for j = 0, SIZE - 1 do
            for k = 0, SIZE - 1 do
                if s[j][k] then
                    covered += 1
                end
            end
        end
        local on = cells(s, p.x, p.y)
        if #on ~= covered then
            problem(string.format("%s at (%d, %d): off the board", what, p.x - center + 2, p.y - center + 2), "navicust_programs", i)
        end
        for _, c in on do
            local at = c[2] * SIZE + c[1]
            if grid[at] ~= nil then
                problem(string.format("%s is over program %d", what, grid[at]), "navicust_programs", i)
                break
            end
            grid[at] = i
        end
    end
end

return {
    run = run,
    validate = validate,
}
