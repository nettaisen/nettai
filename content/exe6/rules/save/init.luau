--!strict
-- What an EXE6 player's save brings to the navi's stats, which nothing else gives (docs/frontend.md §6): the base HP
-- (HP Memories raise it, +0x3E), the Regular memory (RegUps, +0x09) and fighting in the sun (+0x22). The rest of a
-- round's stats are derived as the round is set up, from the navi's fresh stats (`fresh_stats`, its definition):
-- this system first, then the NaviCust's compile (../navicust) and the patch cards (../patch_cards).
--
-- `round_setup` builds a side's stats only when its setup gives the rules something to build them from:
--
-- - a NaviCust to compile (MegaMan): the Regular memory, the sun, the base HP and MegaMan's variant, his base HP in
--   hundreds (+0x2B, which picks his move lag; the console sets it as the battle starts, `sub_800A2F8`); the
--   compile then makes the maximum the base and the HP programs;
-- - a link navi (one with `levels` that doesn't change form) at its navi code's level: the Regular memory, the sun,
--   and its stats as the save's reload gives them at the level (../levels: the base HP its story progress's, the
--   game cleared; the base HP stated here is MegaMan's alone).
--
-- Otherwise the stats are as the setup gives them, whole (a recording's MegaMan, whose block the console's own
-- reload made; a test's).

local fresh_stats = require("./fresh_stats")
local levels = require("./levels")

type Setup = { hp: number, reg_up: number, sun: boolean }

local function round_setup(side: number)
    local stats = battle.navi(side)
    local setup = system.setup() :: Setup
    local navi = stats.navi
    local level = battle.navi_level(side)
    local compiles = battle.navicust(side) ~= nil
    local link_navi = navi.forms == nil and navi.levels ~= nil and level ~= nil
    if not compiles and not link_navi then
        return
    end
    stats.regular_memory = setup.reg_up
    stats.sun = setup.sun
    if link_navi then
        levels.reload(stats, level :: number)
    else
        stats.max_base_hp = setup.hp
        stats.navi_variant = setup.hp // 100
    end
end

return define.system {
    id = "save",
    setup = { hp = "u16", reg_up = "u8", sun = "bool" },
    -- A save that says nothing: a new game's base HP, 100 (MegaMan's fresh HP), and Regular memory (the rules' fresh
    -- stats'); no sun.
    setup_defaults = { hp = 100, reg_up = fresh_stats.reg_up, sun = false },
    hooks = { round_setup = round_setup },
}
