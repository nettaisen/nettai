--!strict
-- EXE5's rules, its one definition of them (docs/design/rules-in-luau.md §2.2): the parts an EXE5 player plays by (plain
-- modules, whose hooks run in this order: @exelib/rules/compose): light and dark MegaMan, EXE5's emotions, Soul Unison,
-- the navis in auto battle Chaos Unison brings, the NaviCust, the patch cards and the folder rules so far; its Team
-- Battle is still to come (docs/design/exe5-map.md §15).
-- Written by tools/exe5/gen_rules.py (bn6battle-verify) from the EXE5 ROM; people own it now.

local compose = require("@exelib/rules/compose")
local auto_battle = require("@self/auto_battle")
local emotion = require("@self/emotion")
local folder = require("@self/folder")
local light_dark = require("@self/light_dark")
local navicust = require("@self/navicust")
local patch_cards = require("@self/patch_cards")
local save = require("@self/save")
local souls = require("@self/souls")

return define.rules(compose.rules({
    -- What the save brings to the navi's stats (MegaMan's base HP, the Regular memory), and a team navi's HP at its
    -- level: first, as the reload builds the stats the NaviCust's compile reads.
    save,
    -- The NaviCust compiled into the navi's stats as the round is set up (EXE6's compile, EXE5's board and bugs).
    navicust,
    -- The patch cards over what the NaviCust made, and the HP (0x0813F97C's).
    patch_cards,
    -- Light and dark MegaMan: the holy panels a dark one clears, the chips each may use.
    light_dark,
    -- EXE5's emotions: Full Synchro on a counter, the emotion-swing bug.
    emotion.part,
    -- Soul Unison: the soul's turns, the revert when they run out (the change is its souls' own action).
    souls.part,
    -- The navis in auto battle a battle brings (a failed Chaos Unison's Dark MegaMan): their AI, their time, their
    -- leave.
    auto_battle.part,
    -- The folder rules: what EXE5's folder editor allows (a tool's question, never the simulation's).
    folder,
}, {
    -- Its rule sections (each a plain table, which the engine reads by its schema) and its roles.
    banners = require("@self/banners"),
    buster = require("@self/buster"),
    chip_use = require("@self/chip_use"),
    custom_screen = require("@self/custom_screen"),
    effects = require("@self/effects"),
    elements = require("@self/elements"),
    flow = require("@self/flow"),
    fresh_stats = require("@self/fresh_stats"),
    link_pick = require("@self/link_pick"),
    math = require("@self/math"),
    navicust = require("@self/navicust/section"),
    panels = require("@self/panels"),
    pools = require("@self/pools"),
    reactions = require("@self/reactions"),
    roles = require("@self/roles"),
    sp_chips = require("@self/sp_chips"),
    status = require("@self/status").rules,
}))
