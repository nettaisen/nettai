--!strict
-- What an EXE6 player's save brings to the navi's stats, which nothing else gives (docs/frontend.md §6): the base HP
-- (HP Memories raise it, +0x3E), the Regular memory (RegUps, +0x09), fighting in the sun (+0x22), and the level of
-- the navi code it received (`level`: 0 to 14, none without one: event flag 0x163, which the init exchange shares,
-- `sub_800B144`, `dword_203CFA0`). A link navi exists only through its code, so it has a level; MegaMan has one when
-- he was received from a code. The engine knows no level: EXE6's rules read it (exe6.navi_level: the custom screen's
-- seal, the reload, what a level gives a link navi in battle, ../by_level). The rest of a
-- round's stats are derived as the round is set up, from the navi's fresh stats (`fresh_stats`, its definition):
-- this module first, then the NaviCust's compile (../navicust) and the patch cards (../patch_cards).
--
-- `round_setup` builds a side's stats only when its setup gives the rules something to build them from:
--
-- - a NaviCust to compile (MegaMan): the Regular memory, the sun, the base HP and MegaMan's variant, his base HP in
--   hundreds (+0x2B, which picks his move lag; the console sets it as the battle starts, `sub_800A2F8`); the
--   compile then makes the maximum the base and the HP programs;
-- - a link navi (one with `levels` that doesn't change form) at its navi code's level: the Regular memory, and its
--   stats as the save's reload gives them at the level (../levels: the base HP its story progress's, the game
--   cleared; the base HP and the sun stated here are MegaMan's alone: the overworld writes his block's sun,
--   `sub_80355EC` from `EnterMap`, a link navi's never).
--
-- Otherwise the stats are as the setup gives them, whole (a recording's MegaMan, whose block the console's own
-- reload made; a test's).

local levels = require("./levels")

type Setup = { hp: number, reg_up: number, sun: boolean, level: number? }

local function round_setup(side: number)
    local stats = battle.navi(side)
    local setup = rules.setup() :: Setup
    local navi = stats.navi
    local level = setup.level
    -- (The navi that changes form compiles a NaviCust, where the setup gives one: ../navicust.)
    local compiles = rules.setup_of(side).navicust_expansions ~= nil and navi.forms ~= nil
    local link_navi = navi.forms == nil and navi.levels ~= nil and level ~= nil
    if not compiles and not link_navi then
        return
    end
    stats.regular_memory = setup.reg_up
    if link_navi then
        levels.reload(stats, level :: number)
    else
        stats.sun = setup.sun
        stats.max_base_hp = setup.hp
        stats.navi_variant = setup.hp // 100
    end
end

-- What every side's save brings at its highest (the rules' `setup_defaults`: no player states these): the base HP
-- (HP Memories to their last) and the Regular memory (RegUps to theirs).
local MAX_HP = 1000
local MAX_REG_UP = 50

-- `validate`'s part: the navi's level as what the save brings allows: a navi code's level, 0 to its last
-- (../levels), which a link navi always has (it exists only through its code: the tools state its last) and MegaMan
-- may leave out (no code received); none for a navi without levels.
local function validate(side: number, problem: (text: string, field: string?, entry: number?) -> ())
    local setup = rules.setup_of(side)
    local navi: NaviDef? = setup.navi
    if navi == nil then
        return
    end
    local name = battle.name(navi)
    local level: number? = setup.level
    local by_level = if navi.levels ~= nil then navi.levels.by_level else nil
    if level ~= nil then
        if by_level == nil then
            problem(string.format("level %d: %s takes no level", level, name), "level")
        elseif level > #by_level - 1 then
            problem(string.format("level %d: a navi code's level is 0 to %d", level, #by_level - 1), "level")
        end
    elseif by_level ~= nil and navi.forms == nil then
        problem(string.format("%s has no level (0 to %d): a link navi exists only through its navi code", name, #by_level - 1), "level")
    end
end

return {
    round_setup = round_setup,
    validate = validate,
    MAX_HP = MAX_HP,
    MAX_REG_UP = MAX_REG_UP,
}
