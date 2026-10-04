--!strict
-- BN5's NaviCust (docs/design/bn5-map.md §15.13): as the round is set up (`round_setup`, before anything reads the
-- stats), a player's NaviCust (`battle.navicust`: the programs placed on its grid) is compiled into the navi's stats,
-- as the original's does (0x0813F97C's 0x0813FA10, for MegaMan alone; BN5's patch cards, 0x08138214, follow it). The
-- compile is BN6's routine for routine (@exelib/navicust/compile): the bugs counted, the stats reset (the setup's are
-- already the navi's fresh stats with what the save keeps: 0x08133DBC keeps the mood, the light/dark value, +0x21 and
-- +0x22, the base HP, the soul, the folder and its three Regular chips, the Regular memory and the HP), the programs'
-- effects, the bugs, the HP. BN5's own:
--
-- - its 5x5 board (./board.luau): no expansions, no frame, any cell of it a neighbor's;
-- - HubBatc (program 27) counts its own bug once more (0x0813F5A8);
-- - its bugs' table (0x08140054; ./programs.luau's BUGS);
-- - no emotion window glitch (its flag, 0x10C1, is read outside battle only);
-- - BugStop (+0x1F) alone stops the bugs here; the patch cards' list can stop or keep them too (0x08137A30), which
--   comes with BN5's patch cards;
-- - the HP (0x0803C13C): in the real world the maximum again and the HP with it, as BN6's; in the cyberworld (the
--   save's area, 0x02002944, from 0x80, or event flag 0x10B2: the setup's `cyberworld`) neither, so the HP is the
--   save's and the maximum the effects' (0x0803C1CC: the base and the HP programs), HubBatc's bug's halving standing.

local board = require("./board")
local compile = require("@exelib/navicust/compile")
local programs = require("./programs")

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
local HUBBATC = "bn5:hubbatc"

-- What the player's save brings (nettai-match from a match's side, the tools from a save): whether it is in the
-- cyberworld (its area, 0x02002944, from 0x80, or event flag 0x10B2), where the compile leaves the HP.
type Setup = { cyberworld: boolean }

-- BN5's compile.
local BN5: NaviCustGame = {
    name = "BN5",
    board = board,
    bugs = programs.BUGS,
    bug_work = BUG_WORK,
    -- 0x0813F5A8: HubBatc (in any color) counts its own bug once more.
    extra = function(part: PlacedProgram, count: (bug: string?, n: number?) -> ())
        if part.program.id == HUBBATC then
            count(part.program.bug)
        end
    end,
    glitch = false,
    keeps_hp = function(_side: number): boolean
        return (system.setup() :: Setup).cyberworld
    end,
}

return define.system {
    id = "bn5:navicust",
    setup = { cyberworld = "bool" },
    hooks = {
        round_setup = function(side: number)
            compile.run(side, BN5)
        end,
    },
}
