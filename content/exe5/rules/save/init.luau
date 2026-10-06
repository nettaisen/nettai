--!strict
-- What an EXE5 player's save brings to the navi's stats, which nothing else gives (docs/frontend.md §6): MegaMan's
-- base HP (HP Memories raise it, +0x3E) and the Regular memory (RegUps, +0x09). (EXE5's block has no sun: its sun is
-- none of the save's.) The rest of a round's stats are derived as the round is set up, from the navi's fresh stats
-- (`fresh_stats`, its definition): this system first, then the NaviCust's compile (../navicust) and the patch cards
-- (../patch_cards).
--
-- `round_setup` builds a side's stats only when its setup gives the rules something to build them from:
--
-- - a NaviCust to compile (MegaMan): the Regular memory, the base HP and MegaMan's variant, his base HP in hundreds
--   (+0x2B; the console sets it as the battle starts); the compile then makes the maximum the base and the HP
--   programs;
-- - a team navi (one with a `story`) at its level: the Regular memory, and its HP (current, maximum and base) as the
--   story leaves it at the level (the game's routine sets it from the table by the story's progress, at a new game's
--   start and from a map script as the story moves on: a level below the last is the progress, and at the last the
--   story is taken as done). The base HP stated here is MegaMan's alone.
--
-- Otherwise the stats are as the setup gives them, whole (a recording's MegaMan without a recorded NaviCust; a
-- test's).

local fresh_stats = require("./fresh_stats")

type Setup = { hp: number, reg_up: number }

-- A team navi's HP at `level` by its story (none past its last level).
local function story_hp(story: { hp: { number }, max_level: number }, level: number): number?
    if level < story.max_level then
        return story.hp[level + 1]
    elseif level == story.max_level then
        return story.hp[#story.hp]
    end
    return nil
end

local function round_setup(side: number)
    local stats = battle.navi(side)
    local setup = system.setup() :: Setup
    local story = stats.navi.story
    local level = battle.navi_level(side)
    local compiles = battle.navicust(side) ~= nil
    local team_navi = story ~= nil and level ~= nil
    if not compiles and not team_navi then
        return
    end
    stats.regular_memory = setup.reg_up
    if story ~= nil and level ~= nil then
        local hp = story_hp(story, level)
        if hp ~= nil then
            stats.max_base_hp = hp
            stats.max_hp = hp
            stats.hp = hp
        end
    else
        stats.max_base_hp = setup.hp
        stats.navi_variant = setup.hp // 100
    end
end

return define.system {
    id = "save",
    setup = { hp = "u16", reg_up = "u8" },
    -- A save that says nothing: a new game's base HP, 100 (MegaMan's fresh HP), and Regular memory (the rules' fresh
    -- stats').
    setup_defaults = { hp = 100, reg_up = fresh_stats.reg_up },
    hooks = { round_setup = round_setup },
}
