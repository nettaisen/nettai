--!strict
-- EXE5's NaviCust (docs/design/exe5-map.md §15.13): as the round is set up (`round_setup`, before anything reads the
-- stats), a player's NaviCust (`battle.navicust`: the programs placed on its grid) is compiled into the navi's stats,
-- as the original's does (0x0813F97C's 0x0813FA10, for MegaMan alone; EXE5's patch cards, 0x08138214, follow it). The
-- compile is EXE6's routine for routine (@exelib/navicust/compile): the bugs counted, the stats reset (the setup's are
-- already the navi's fresh stats with what the save keeps: 0x08133DBC keeps the mood, the light/dark value, +0x21 and
-- +0x22, the base HP, the soul, the folder and its three Regular chips, the Regular memory and the HP), the programs'
-- effects, the bugs, the HP. EXE5's own:
--
-- - its board (./board.luau): 4x4, 5x4 or 5x5 by the save's ExpMemry (which the compile doesn't read: only placing a
--   program does), no frame, any cell of the grid a neighbor's;
-- - HubBatc (program 27) counts its own bug once more (0x0813F5A8);
-- - its bugs' table (0x08140054; ./programs.luau's BUGS);
-- - the emotion window's glitch: its flag, 0x10C1 (set when a bug applies, 0x08140040), which the window's start
--   (0x0801AF14, through 0x0813F650) reads when the save has no patch cards (with cards, the cards' own flag:
--   ../patch_cards);
-- - BugStop (+0x1F) stops the bugs unless the patch cards say (0x0813FA48 asks 0x08137A30 first): a
--   card's BugStop effect (the last card's) stops them at 1 and keeps them else;
-- - the HP (0x0803C13C): the maximum again and the HP with it, as EXE6's. That is the original's ending in the real
--   world, and the one a battle here has. Its other, for a console in the cyberworld (the save's area, 0x02002944,
--   from 0x80, or event flag 0x10B2), leaves both: the HP the save's, and the maximum the effects' (0x0803C1CC: the
--   base and the HP programs) with HubBatc's bug's halving standing. No link battle can tell the two apart: its start
--   puts every navi at its maximum (`init_hp`: the settings' keep-HP effect is a story battle's), and the maximum
--   differs only with HubBatc and an HP program on one board, which no board holds (HubBatc's one place leaves an HP
--   program no room, and the editor refuses an overlap, 0x0813F2A4; docs/design/exe5-map.md §15.13). So a side
--   states no such thing, and the rules have the one ending.

local board = require("@self/board")
local compile = require("@exelib/navicust/compile")
local programs = require("@self/programs")

-- What the panel, custom and buster bugs write: the cracked trail's steps in 8 by the bug's level (+0x13), the
-- custom screen's damage more by its level (+0x54), and the buster's blank and charged shots of 16 at any level
-- (+0x14, +0x15).
local PANEL_STEPS: { number } = { 2, 4, 8 }
local CUSTOM_DAMAGE: { number } = { 20, 40, 80 }
local BLANKS, CHARGED = 4, 2

-- Each bug at its level, 1 to 3 (0x08140054's routines).
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
        stats.custom_damage = bit32.band(stats.custom_damage + CUSTOM_DAMAGE[level], 0xFFFF)
    end,
    encounter = function(stats, _)
        stats.encounters = 1
    end,
    result = function(stats, _)
        stats.chip_drops = 1
    end,
    buster = function(stats, _)
        stats.buster_blanks = BLANKS
        stats.buster_charged = CHARGED
    end,
    support = function(stats, _)
        stats.support_bug = true
    end,
    hp = function(stats, _)
        stats.hit_status = 3
    end,
    hub = function(stats, _, hp)
        -- 0x08140248: the maximum the base and half the HP programs, the HP no more than it (in the real world the
        -- HP's routine, 0x0803C13C, makes both whole again as the compile ends).
        local most = stats.max_base_hp + bit32.rshift(hp, 1)
        stats.max_hp = bit32.band(most, 0xFFFF)
        if most < stats.hp then
            stats.hp = stats.max_hp
        end
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
-- - An HP drain, the fight's or the custom screen's: its level is the stat's own, 1 to 7, which the card adds to.
-- - The custom screen's damage: the level whose damage the card's is (`CUSTOM_DAMAGE`: 20, 40, 80); any other, the
--   amount, the damage itself. (EXE5's cards are at 50, 100 and 200: none a level's.)
-- - The blank shots: level 1 where the card's blank and charged shots are the NaviCust's bug's (4 and 2, the same
--   at every level); any other, the amount: the blank shots of the buster's 16.
-- - A trail: the cracked one at a level's steps (`PANEL_STEPS`), that level; any other, and every other trail, the
--   amount: the steps in 8 that leave it (the stat is a step's chance in 8).
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
        return if blanks == BLANKS and any.charged == CHARGED
            then { name = kind, level = 1 }
            else { name = kind, amount = blanks, of = 16 }
    elseif kind == "custom_damage" then
        local level = table.find(CUSTOM_DAMAGE, e.amount or 0)
        return if level ~= nil then { name = kind, level = level } else { name = kind, amount = e.amount }
    elseif kind == "status_bug" then
        return { name = kind, level = math.min(e.amount or 0, 2) }
    elseif kind == "hp_drain" or kind == "custom_hp_drain" then
        return { name = kind, level = e.amount }
    end
    return { name = kind, level = 1 }
end

-- EXE5's compile.
local EXE5: NaviCustGame = {
    name = "EXE5",
    board = board,
    bugs = programs.BUGS,
    bug_names = programs.BUG_NAMES,
    bug_work = BUG_WORK,
    -- 0x0813F5A8: a program that says so (HubBatc, in any color) counts its own bug once more.
    extra = function(part: PlacedProgram, count: (bug: string?, n: number?) -> ())
        if part.program.counts_own_bug then
            count(part.program.bug)
        end
    end,
    glitch = true,
    -- 0x08137A30: the cards' BugStop (the block as the cards' last application left it, whose cards these are): at 1
    -- the bugs stop, else they stay; no card's, the NaviCust's own.
    bug_stop = function(side: number): boolean?
        local value: number? = nil
        for _, c in rules.setup_of(side).patch_cards :: { PatchCard } do
            for _, e in c.effects do
                if e.kind == "bug_stopper" then
                    value = e.amount
                    break
                end
            end
        end
        if value == nil then
            return nil
        end
        return value == 1
    end,
}

return {
    card_bug = card_bug,
    -- What is wrong with the NaviCust on EXE5's board (@exelib/navicust/compile's check).
    validate = function(side: number, problem: (p: SetupProblem) -> ())
        local setup = rules.setup_of(side)
        compile.check(EXE5, setup.navicust_expansions, setup.navicust_programs, battle.navi(side).navi.forms ~= nil, problem)
    end,
    -- `overview`'s part: the programs at work that give the navi an ability, and the NaviCust's bugs.
    overview = function(side: number, out: SetupOverview)
        local setup = rules.setup_of(side)
        compile.overview(side, EXE5, setup.navicust_expansions, setup.navicust_programs, out)
    end,
    -- As the round is set up: the programs compiled into the navi's stats.
    round_setup = function(side: number)
        local setup = rules.setup_of(side)
        compile.run(side, EXE5, setup.navicust_expansions, setup.navicust_programs)
    end,
}
