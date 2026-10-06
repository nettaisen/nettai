--!strict
-- What an EXE5 player's save brings to the navi's stats, which nothing else gives (docs/frontend.md §6): MegaMan's
-- base HP (HP Memories raise it, +0x3E), the Regular memory (RegUps, +0x09), and a team navi's level (`level`: the
-- count of the save's story flags 0x300 to 0x305, 0 to the navi's story's last, which the init exchange sends; none
-- for MegaMan, who takes none). The engine knows no level: EXE5's rules read it (exe5.navi_level, ../api: the team
-- navis' damage rows, lib/navi_level). (EXE5's
-- block has no sun: its sun is none of the save's.) The rest of a round's stats are derived as the round is set up,
-- from the navi's fresh stats
-- (`fresh_stats`, its definition): this module first, then the NaviCust's compile (../navicust) and the patch cards
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

type Setup = { hp: number, reg_up: number, level: number? }

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
    local setup = rules.setup() :: Setup
    local story = stats.navi.story
    local level = setup.level
    -- (The navi that changes form compiles a NaviCust, where the setup gives one: ../navicust.)
    local compiles = rules.setup_of(side).navicust_expansions ~= nil and stats.navi.forms ~= nil
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

-- The base HP a setup that says nothing has (a new game's: MegaMan's fresh HP).
local NEW_GAME_HP = 100

-- `validate`'s part: the navi's level and the base HP as what the save brings allows: a team navi's level, its
-- story's (0 to its last), always stated; none for a navi without a story. The base HP is MegaMan's (the navi that
-- compiles a NaviCust): a team navi's HP is its level's, so a side of one that states another than a new game's
-- states what has no effect.
local function validate(side: number, problem: (text: string, field: string?, entry: number?) -> ())
    local setup = rules.setup_of(side)
    local navi: NaviDef? = setup.navi
    if navi == nil then
        return
    end
    local name = battle.name(navi)
    local level: number? = setup.level
    local story = navi.story
    if level ~= nil then
        if story == nil then
            problem(string.format("level %d: %s takes no level", level, name), "level")
        elseif level > story.max_level then
            problem(string.format("level %d: %s's level is 0 to %d", level, name, story.max_level), "level")
        end
    elseif story ~= nil then
        problem(string.format("%s has no level (0 to %d)", name, story.max_level), "level")
    end
    if navi.forms == nil and setup.hp ~= NEW_GAME_HP then
        problem(string.format("hp: %s's HP is its level's; a side states the base HP of the navi that compiles a NaviCust", name), "hp")
    end
end

return {
    round_setup = round_setup,
    validate = validate,
    NEW_GAME_HP = NEW_GAME_HP,
}
