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

-- What the panel bug and the buster bug write by their level: the cracked trail's steps in 8 (+0x13), and the
-- buster's blank shots of 16 (+0x14; its charged shots, +0x15, are the level itself).
local PANEL_STEPS: { number } = { 2, 3, 4 }
local BLANKS: { number } = { 6, 10, 13 }

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
        stats.panel_trail_level = PANEL_STEPS[level]
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
        stats.buster_blanks = BLANKS[level]
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

-- A patch card's bug effect as a tool lists it (../patch_cards' `overview`): the bug it is, by the effect's own key
-- in the locales (its kind, and its trail or status), which is the name the NaviCust's bug of the same stat has, and
-- how much of it: its level, or its amount, never both.
--
-- - The step bug, the emotion bug and a hit's status are on or off: level 1.
-- - The battle start's bug: level 1 (+0x1A = 9, the NaviCust's `status`) or 2 (10, `status-strong`).
-- - An HP drain, the fight's or the custom screen's: its level is the stat's own, 1 to 7, which the card adds to (so
--   the caller adds the NaviCust's bug's level to the fight's).
-- - The custom screen's damage: the amount, the damage itself. No level: the NaviCust brings no such bug in EXE6
--   (EXE5's does, at 20, 40 and 80, and EXE6's cards are at 20, 40 and 50).
-- - The blank shots: the level whose blank and charged shots the card's are (`BLANKS` and the level itself); any
--   other, the amount: the blank shots of the buster's 16. (EXE6's cards are all 4 blanks, less than level 1's 6.)
-- - A trail: the cracked one at a level's steps (`PANEL_STEPS`), that level; any other, and every other trail, the
--   amount: the steps in 8 that leave it (the stat is a step's chance in 8). (EXE6's cards crack at 1 in 8, less than
--   level 1's 2.)
local function card_bug(e: PatchCardEffect): SetupBug
    local kind = e.kind
    local any = e :: any
    if kind == "panel_trail" then
        local steps: number = any.level
        local level: number? = if any.trail == "crack" then table.find(PANEL_STEPS, steps) else nil
        local name: string = "panel_trail." .. any.trail
        return if level ~= nil then { name = name, level = level } else { name = name, amount = steps, of = 8 }
    elseif kind == "hit_status" then
        return { name = "hit_status." .. any.status, level = 1 }
    elseif kind == "buster_blanks" then
        local blanks: number = any.blanks
        local level = table.find(BLANKS, blanks)
        return if level ~= nil and level == any.charged
            then { name = kind, level = level }
            else { name = kind, amount = blanks, of = 16 }
    elseif kind == "custom_damage" then
        return { name = kind, amount = e.amount }
    elseif kind == "status_bug" then
        return { name = kind, level = math.min(e.amount or 0, 2) }
    elseif kind == "hp_drain" or kind == "custom_hp_drain" then
        return { name = kind, level = e.amount }
    end
    return { name = kind, level = 1 }
end

-- EXE6's compile: its board, its bugs, and the emotion window's glitch any bug sets (flag 0x1720).
local EXE6: NaviCustGame = {
    name = "EXE6",
    board = board,
    bugs = programs.BUGS,
    bug_work = BUG_WORK,
    bug_names = programs.BUG_NAMES,
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
local function validate(side: number, problem: (p: SetupProblem) -> ())
    local setup = rules.setup_of(side)
    compile.check(EXE6, setup.navicust_expansions, setup.navicust_programs, battle.navi(side).navi.forms ~= nil, problem)
end

-- `overview`'s part: the programs at work that give the navi an ability, and the NaviCust's bugs.
local function overview(side: number, out: SetupOverview)
    local setup = rules.setup_of(side)
    compile.overview(side, EXE6, setup.navicust_expansions, setup.navicust_programs, out)
end

return {
    round_setup = run,
    validate = validate,
    overview = overview,
    card_bug = card_bug,
    redeal = redeal,
}
