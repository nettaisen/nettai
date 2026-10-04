--!strict
-- BN5's stock rules (docs/design/rules-in-luau.md §2.2): the systems a BN5 player plays by: light and dark
-- MegaMan, BN5's emotions, Soul Unison, the computer navis Chaos Unison brings, the NaviCust, the patch cards and the
-- folder rules so far; its Team Battle is still to come as a system (docs/design/bn5-map.md §15).
-- Written by tools/bn5/gen_rules.py (bn6battle-verify) from the BN5 ROM; people own it now.

local computer_navi = require("@self/computer-navi")
local emotion = require("@self/emotion")
local folder = require("@self/folder")
local light_dark = require("@self/light-dark")
local navicust = require("@self/navicust")
local patch_cards = require("@self/patch-cards")
local souls = require("@self/souls")

return define.ruleset {
    id = "stock",
    stock = true,
    systems = {
        -- The NaviCust compiled into the navi's stats as the round is set up (BN6's compile, BN5's board and bugs).
        navicust,
        -- The patch cards over what the NaviCust made, and the HP (0x0813F97C's).
        patch_cards,
        -- Light and dark MegaMan: the holy panels a dark one clears, the chips each may use.
        light_dark,
        -- BN5's emotions: Full Synchro on a counter, the emotion-swing bug.
        emotion.system,
        -- Soul Unison: the soul's turns, the revert when they run out (the change is its souls' own action).
        souls.system,
        -- The computer navis a battle brings (a failed Chaos Unison's Dark MegaMan): their AI, their time, their
        -- leave.
        computer_navi.system,
        -- The folder rules: what BN5's folder editor allows (a tool's question, never the simulation's).
        folder,
    },
    -- Its rule sections (each a plain table, which the engine reads by its schema) and its roles.
    banners = require("@self/banners"),
    buster = require("@self/buster"),
    chip_use = require("@self/chip-use"),
    custom_screen = require("@self/custom-screen"),
    effects = require("@self/effects"),
    elements = require("@self/elements"),
    flow = require("@self/flow"),
    math = require("@self/math"),
    navicust = require("@self/navicust/section"),
    panels = require("@self/panels"),
    pools = require("@self/pools"),
    reactions = require("@self/reactions"),
    roles = require("@self/roles"),
    sp_chips = require("@self/sp-chips"),
    status = require("@self/status").rules,
}
