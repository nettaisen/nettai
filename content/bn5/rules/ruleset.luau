--!strict
-- BN5's stock rules (docs/design/rules-in-luau.md §2.2): the systems a BN5 player plays by: light and dark
-- MegaMan, BN5's emotions, Soul Unison, the computer navis Chaos Unison brings and the folder rules so far; its Team
-- Battle is still to come as a system (docs/design/bn5-map.md §15).
-- Written by tools/bn5/gen_rules.py (bn6battle-verify) from the BN5 ROM; people own it now.

local computer_navi = require("./computer-navi/system")
local emotion = require("./emotion/system")
local folder = require("./folder/system")
local light_dark = require("./light-dark/system")
local souls = require("./souls/system")

return define.ruleset {
    id = "bn5:stock",
    stock = true,
    systems = {
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
}
