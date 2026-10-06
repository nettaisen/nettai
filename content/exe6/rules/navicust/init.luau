--!strict
-- EXE6's NaviCust (docs/design/navicust.md): as the round is set up
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
--    fresh stats with what the save keeps (../save wrote the base HP, the
--    Regular memory and the sun), so no work here.
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
-- A MegaMan received from a navi code (event flag 0x163, rules/save's
-- `level`) then gets his level's gains over what his NaviCust made
-- (`reloadCurNaviStatBoosts`: `sub_8121154` after `sub_813C458`;
-- docs/engine/link-navis.md §4), and the HP its maximum again.
--
-- A program's color, shape and whether it is a plus part are the engine's
-- (its definition's); its `bug`, its `exclusive` and its `effects` this
-- part's (./programs.luau). A program `anywhere` (no EXE6 program is one)
-- works on the command line or off it and is never bugged by where it is.
-- The compile is the one EXE5's shares (@exelib/navicust/compile); EXE6's are
-- its board (its frame, where a neighbor counts), its bugs and the emotion
-- window's glitch.

local board = require("@self/board")
local compile = require("@exelib/navicust/compile")
local exe6 = require("./api")
local levels = require("./levels")
local programs = require("@self/programs")

-- Each bug at its level, 1 to 3 (`byte_813CC18`'s routines).
local BUG_WORK: { [string]: (stats: Navi, level: number, hp: number) -> () } = {
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

-- EXE6's compile: its board, its bugs, and the emotion window's glitch any bug sets (flag 0x1720).
local EXE6: NaviCustGame = {
    name = "EXE6",
    board = board,
    bugs = programs.BUGS,
    bug_work = BUG_WORK,
    glitch = true,
}

-- `round_setup`: `side`'s NaviCust compiled into its stats; MegaMan from a
-- navi code then gets his level's gains, and the HP its maximum.
local function run(side: number)
    local setup = rules.setup_of(side)
    if not compile.run(side, EXE6, setup.navicust_expansions, setup.navicust_programs) then
        return
    end
    local level = exe6.navi_level(side)
    if level ~= nil then
        local stats = battle.navi(side)
        levels.add(stats, level)
        stats.hp = stats.max_hp
    end
end

-- ChpShufl's re-deal button (`sub_80280E0`), two cells wide on slots 8
-- and 9 (where DustCross's scrap goes first), usable once: A deals the
-- chips not picked again (`sub_8028DD6`).
local redeal: ButtonSpec = {
    slot = 8,
    cells = 2,
    uses = 1,
    right = 11,
    left = 7,
    shown = function(side: number): boolean
        local stats = battle.navi(side)
        return stats.navi.forms ~= nil and stats.chip_shuffle
    end,
    pressed = function(side: number)
        if custom.cursor_state(side) == "selectable" then
            custom.play(side, "redeal")
            custom.redeal(side)
        else
            custom.refuse(side)
        end
    end,
}

-- `validate`'s part: the NaviCust on EXE6's board (@exelib/navicust/compile's check).
local function validate(side: number, problem: (text: string, field: string?, entry: number?) -> ())
    local setup = rules.setup_of(side)
    compile.check(EXE6, setup.navicust_expansions, setup.navicust_programs, battle.navi(side).navi.forms ~= nil, problem)
end

return {
    round_setup = run,
    validate = validate,
    redeal = redeal,
}
