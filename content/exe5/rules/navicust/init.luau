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
-- - BugStop (+0x1F) stops the bugs unless the patch cards say (0x0813FA48 asks 0x08137A30 first): a switched-on
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
        stats.panel_trail_level = ({ 2, 4, 8 })[level]
    end,
    custom = function(stats, level)
        stats.custom_damage = bit32.band(stats.custom_damage + ({ 20, 40, 80 })[level], 0xFFFF)
    end,
    encounter = function(stats, _)
        stats.encounters = 1
    end,
    result = function(stats, _)
        stats.chip_drops = 1
    end,
    buster = function(stats, _)
        stats.buster_blanks = 4
        stats.buster_charged = 2
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

-- HubBatc (program 27), which bugs itself.
local HUBBATC = "hubbatc"

-- EXE5's compile.
local EXE5: NaviCustGame = {
    name = "EXE5",
    board = board,
    bugs = programs.BUGS,
    bug_work = BUG_WORK,
    -- 0x0813F5A8: HubBatc (in any color) counts its own bug once more.
    extra = function(part: PlacedProgram, count: (bug: string?, n: number?) -> ())
        if part.program.id == HUBBATC then
            count(part.program.bug)
        end
    end,
    glitch = true,
    -- 0x08137A30: the cards' BugStop (the block as the cards' last application left it, whose cards these are): at 1
    -- the bugs stop, else they stay; no card's, the NaviCust's own.
    bug_stop = function(side: number): boolean?
        local value: number? = nil
        for _, c in battle.patch_cards(side) do
            if c.enabled then
                for _, e in c.card.effects do
                    if e.kind == "bug_stopper" then
                        value = e.amount
                        break
                    end
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
    -- As the round is set up: the programs compiled into the navi's stats.
    round_setup = function(side: number)
        compile.run(side, EXE5)
    end,
}
